use crate::api::Result;
use crate::fre_update_policy::{
	MAX_UPDATE_BYTES, accept_size, validate_download, verify_signature,
};
use std::sync::{Arc, Mutex};
use tauri::{Manager, ResourceId, Runtime, Webview};
use tauri_plugin_http::reqwest::Client;
use tauri_plugin_updater::Update;
use theseus::{
	LoadingBarType, emit_loading, init_loading, launcher_user_agent,
};

#[derive(Default)]
pub struct PendingUpdateData(pub Mutex<Option<(Arc<Update>, Vec<u8>)>>);
static DOWNLOAD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn client() -> anyhow::Result<Client> {
	Ok(Client::builder()
		.user_agent(launcher_user_agent())
		.connect_timeout(std::time::Duration::from_secs(20))
		.timeout(std::time::Duration::from_secs(5 * 60))
		.build()?)
}

#[tauri::command]
pub async fn get_update_size<R: Runtime>(
	webview: Webview<R>,
	rid: ResourceId,
) -> Result<Option<u64>> {
	let update = webview.resources_table().get::<Update>(rid)?;
	validate_download(&update.download_url, &update.version)?;
	let response = client()?
		.head(update.download_url.clone())
		.timeout(std::time::Duration::from_secs(20))
		.send()
		.await?
		.error_for_status()?;
	let size = response.content_length();
	if let Some(size) = size {
		accept_size(size)?;
	}
	Ok(size)
}

#[tauri::command]
pub async fn enqueue_update_for_installation<R: Runtime>(
	webview: Webview<R>,
	rid: ResourceId,
) -> Result<()> {
	let _download = DOWNLOAD
		.try_lock()
		.map_err(|_| anyhow::anyhow!("An update is already downloading"))?;
	let pending = webview.state::<PendingUpdateData>();
	if pending.0.lock().unwrap().is_some() {
		return Err(
			anyhow::anyhow!("An update is already ready to install").into()
		);
	}
	let update = webview.resources_table().get::<Update>(rid)?;
	validate_download(&update.download_url, &update.version)?;
	let progress = init_loading(
		LoadingBarType::LauncherUpdate {
			version: update.version.clone(),
			current_version: update.current_version.clone(),
		},
		1.0,
		"Downloading FRE LAUNCH update...",
	)
	.await?;
	let mut response = client()?
		.get(update.download_url.clone())
		.send()
		.await?
		.error_for_status()?;
	let expected = response.content_length();
	if let Some(size) = expected {
		accept_size(size)?;
	}
	let mut bytes = Vec::new();
	while let Some(chunk) = response.chunk().await? {
		let size = bytes.len().saturating_add(chunk.len()) as u64;
		if size > MAX_UPDATE_BYTES {
			return Err(anyhow::anyhow!(
				"Update exceeds the allowed installer size"
			)
			.into());
		}
		bytes.extend_from_slice(&chunk);
		if let Some(total) = expected {
			emit_loading(&progress, chunk.len() as f64 / total as f64, None)?;
		}
	}
	accept_size(bytes.len() as u64)?;
	if expected.is_some_and(|expected| expected != bytes.len() as u64) {
		return Err(anyhow::anyhow!("Incomplete update download").into());
	}
	if !bytes.starts_with(b"MZ") {
		return Err(anyhow::anyhow!("Update is not a Windows installer").into());
	}
	// Only verified bytes are ever handed to the native NSIS updater.
	verify_signature(
		&bytes,
		&update.signature,
		crate::fre_update_channel::PUBLIC_KEY,
	)?;
	pending.0.lock().unwrap().replace((update, bytes));
	Ok(())
}

#[tauri::command]
pub fn remove_enqueued_update<R: Runtime>(webview: Webview<R>) {
	webview
		.state::<PendingUpdateData>()
		.0
		.lock()
		.unwrap()
		.take();
}

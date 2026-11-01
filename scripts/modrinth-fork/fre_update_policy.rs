use anyhow::{Context, bail};
use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use url::Url;

pub const MAX_UPDATE_BYTES: u64 = 256 * 1024 * 1024;

pub fn validate_download(url: &Url, version: &str) -> anyhow::Result<()> {
	let expected = format!(
		"/{}/releases/download/v{version}/FRE-LAUNCH-Windows-x64-Setup.exe",
		crate::fre_update_channel::REPOSITORY
	);
	if url.scheme() != "https"
		|| url.host_str() != Some("github.com")
		|| !url.username().is_empty()
		|| url.password().is_some()
		|| url.port_or_known_default() != Some(443)
		|| url.path() != expected
		|| url.query().is_some()
		|| url.fragment().is_some()
	{
		bail!("Update is not a FRE LAUNCH Windows release");
	}
	Ok(())
}

pub fn verify_signature(
	data: &[u8],
	signature: &str,
	public_key: &str,
) -> anyhow::Result<()> {
	let decode = |value: &str| -> anyhow::Result<String> {
		String::from_utf8(
			base64::engine::general_purpose::STANDARD.decode(value.trim())?,
		)
		.context("Invalid updater signature encoding")
	};
	let key = PublicKey::decode(&decode(public_key)?)?;
	let signature = Signature::decode(&decode(signature)?)?;
	key.verify(data, &signature, false)
		.context("Update signature verification failed")?;
	Ok(())
}

pub fn accept_size(size: u64) -> anyhow::Result<()> {
	if size == 0 || size > MAX_UPDATE_BYTES {
		bail!("Update exceeds the allowed installer size");
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn only_our_exact_release_can_be_downloaded() {
		let url = format!(
			"https://github.com/{}/releases/download/v0.5.9/FRE-LAUNCH-Windows-x64-Setup.exe",
			crate::fre_update_channel::REPOSITORY
		);
		assert!(validate_download(&Url::parse(&url).unwrap(), "0.5.9").is_ok());
		for url in [
			url.replace("https:", "http:"),
			url.replace("github.com", "github.com.evil.test"),
			url.replace("Freeze0102", "another-owner"),
			format!("{url}?redirect=1"),
		] {
			assert!(
				validate_download(&Url::parse(&url).unwrap(), "0.5.9").is_err()
			);
		}
		assert!(accept_size(0).is_err());
		assert!(accept_size(MAX_UPDATE_BYTES + 1).is_err());
		assert!(accept_size(1234).is_ok());
	}

	#[test]
	fn signed_fixture_rejects_corruption_and_wrong_key() {
		let fixture: serde_json::Value =
			serde_json::from_str(include_str!("fre-update-test.json")).unwrap();
		let data = fixture["data"].as_str().unwrap().as_bytes();
		let signature = fixture["signature"].as_str().unwrap();
		let key = crate::fre_update_channel::PUBLIC_KEY;
		verify_signature(data, signature, key).unwrap();
		assert!(verify_signature(b"tampered", signature, key).is_err());
		assert!(verify_signature(data, "unsigned", key).is_err());
		assert!(verify_signature(data, signature, "wrong-key").is_err());
	}
}

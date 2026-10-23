//! Reads local Windows Error Reporting (WER) files for FRE LAUNCH only.
//! No uploads, registry changes, event-log privileges or third-party programs.
use std::fs;

fn extract_fields(report: &str) -> String {
    report.lines()
        .filter(|line| {
            let field=line.split('=').next().unwrap_or("").trim();
            matches!(field,"EventType" | "EventTime" | "FriendlyEventName" | "AppName" |
                "NsAppName" | "ReportIdentifier" | "Response.BucketId" | "ReportStatus")
                || (field.starts_with("Sig[") && (field.ends_with(".Name") || field.ends_with(".Value")))
        })
        .take(48)
        .map(str::trim)
        .collect::<Vec<_>>().join("\n")
}

fn decode_report(bytes: &[u8]) -> String {
    // Report.wer is commonly UTF-16LE, but some Windows builds emit UTF-8.
    let utf16 = bytes.starts_with(&[0xff,0xfe])
        || (bytes.len() >= 8 && bytes[1]==0 && bytes[3]==0);
    if utf16 {
        let skip=if bytes.starts_with(&[0xff,0xfe]) {2} else {0};
        let words:Vec<u16>=bytes[skip..].chunks_exact(2)
            .map(|pair|u16::from_le_bytes([pair[0],pair[1]])).collect();
        String::from_utf16_lossy(&words)
    }else{
        String::from_utf8_lossy(bytes).into_owned()
    }
}

// Scanning Windows WER directories can touch many files. Run outside
// the WebView2/Tauri UI event thread so opening diagnostics never stalls it.
#[tauri::command]
pub async fn read_windows_crash_report() -> Result<String, String> {
    tokio::task::spawn_blocking(read_report_file)
        .await.map_err(|e|format!("WER scan task interrupted: {e}"))?
}

fn read_report_file() -> Result<String, String> {
    #[cfg(not(windows))]
    {Ok(String::new())}

    #[cfg(windows)]
    {
        use std::{path::PathBuf, time::UNIX_EPOCH};
        let mut roots=Vec::<PathBuf>::new();
        if let Some(local)=std::env::var_os("LOCALAPPDATA") {
            for subdir in ["ReportArchive","ReportQueue"] {
                roots.push(PathBuf::from(&local).join("Microsoft").join("Windows").join("WER").join(subdir));
            }
        }
        if let Some(program)=std::env::var_os("PROGRAMDATA") {
            roots.push(PathBuf::from(program).join("Microsoft").join("Windows").join("WER").join("ReportArchive"));
        }
        let mut candidates=Vec::<(u64,PathBuf)>::new();
        for root in roots {
            let Ok(folders)=fs::read_dir(&root) else {continue;};
            for entry in folders.flatten().take(2000) {
                let name=entry.file_name().to_string_lossy().to_ascii_lowercase();
                if !name.starts_with("appcrash_fre-launch")
                    && !name.starts_with("apphang_fre-launch") {continue;}
                let Ok(kind)=entry.file_type() else {continue;};
                if !kind.is_dir() || kind.is_symlink() {continue;}
                let path=entry.path().join("Report.wer");
                let Ok(meta)=fs::symlink_metadata(&path) else {continue;};
                if !meta.is_file() || meta.file_type().is_symlink() ||
                    meta.len() > 256 * 1024 {continue;}
                let when=meta.modified().ok()
                    .and_then(|time|time.duration_since(UNIX_EPOCH).ok())
                    .map(|t|t.as_secs()).unwrap_or(0);
                candidates.push((when,path));
            }
        }
        candidates.sort_by(|a,b|b.0.cmp(&a.0));
        for (_,path) in candidates.into_iter().take(12) {
            let Ok(bytes)=fs::read(&path) else {continue;};
            let decoded=decode_report(&bytes);
            if !decoded.to_ascii_lowercase().contains("fre-launch") {continue;}
            let filtered=extract_fields(&decoded);
            if filtered.is_empty(){continue;}
            return Ok(format!("WINDOWS ERROR REPORT / FRE LAUNCH\n{filtered}"));
        }
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_only_app_crash_signature() {
        let data="EventType=APPCRASH\nSig[0].Name=Application Name\nSig[0].Value=fre-launch.exe\nSig[3].Value=WebView2Loader.dll\nSecretData=do not copy\n";
        let filtered=extract_fields(data);
        assert!(filtered.contains("WebView2Loader.dll"));
        assert!(filtered.contains("fre-launch.exe"));
        assert!(!filtered.contains("SecretData"));
    }
    #[test]
    fn accepts_utf16_and_utf8_wer_reports() {
        let sample="EventType=APPCRASH\nSig[0].Value=fre-launch.exe";
        assert_eq!(decode_report(sample.as_bytes()),sample);
        let mut utf16=vec![0xff,0xfe];
        for word in sample.encode_utf16() {utf16.extend_from_slice(&word.to_le_bytes());}
        assert_eq!(decode_report(&utf16),sample);
    }
}

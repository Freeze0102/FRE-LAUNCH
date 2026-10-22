use serde::Serialize;
use std::{collections::{HashSet,VecDeque}, env, fs, path::{Path,PathBuf}, process::Command};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntime {
    pub path: String,
    pub version: String,
    pub major: u32,
}

fn java_major(line: &str) -> Option<u32> {
    let quoted = line.split('"').nth(1)?;
    let versions: Vec<&str> = quoted.split('.').collect();
    if versions.first() == Some(&"1") {versions.get(1)?.parse().ok()}
    else {versions.first()?.parse().ok()}
}

fn run_java(executable: &Path) -> Option<JavaRuntime> {
    let mut cmd = Command::new(executable);
    cmd.arg("-version");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let result = cmd.output().ok()?;
    if !result.status.success() {return None;}
    let output = String::from_utf8_lossy(&result.stderr);
    let version = output.lines().next()?.to_string();
    Some(JavaRuntime {path:executable.to_string_lossy().into_owned(),major:java_major(&version)?,version})
}

fn discover_dir(base: &Path, depth: usize, paths: &mut Vec<PathBuf>) {
    if !base.is_dir() {return;}
    let mut queue = VecDeque::from([(base.to_path_buf(),0usize)]);
    let mut visited = 0usize;
    while let Some((dir,level)) = queue.pop_front() {
        if visited >= 600 || paths.len() >= 70 {break;}
        visited+=1;
        let bin = dir.join("bin").join(if cfg!(windows) {"java.exe"} else {"java"});
        if bin.is_file() {paths.push(bin);}
        if level >= depth {continue;}
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(kind) = entry.file_type() {
                    if kind.is_dir() && !kind.is_symlink() {queue.push_back((path,level+1));}
                }
            }
        }
    }
}

#[tauri::command]
pub fn list_java_runtimes() -> Vec<JavaRuntime> {
    let mut candidate = Vec::<PathBuf>::new();
    if let Ok(java_home) = env::var("JAVA_HOME") {
        candidate.push(Path::new(&java_home).join("bin").join(if cfg!(windows) {"java.exe"} else {"java"}));
    }
    // PATH resolution is used as a last resort and listed explicitly.
    candidate.push(PathBuf::from(if cfg!(windows) {"java.exe"} else {"java"}));

    #[cfg(windows)]
    for root_key in ["ProgramFiles","ProgramFiles(x86)","LOCALAPPDATA"] {
        if let Ok(base) = env::var(root_key) {
            let base = Path::new(&base);
            for dirname in ["Java","Eclipse Adoptium","Microsoft","Zulu","Programs\\Eclipse Adoptium","Programs\\Java","Programs\\Microsoft"] {
                discover_dir(&base.join(dirname),2,&mut candidate);
            }
        }
    }
    if let Ok(appdata) = env::var("APPDATA") {
        discover_dir(&Path::new(&appdata).join(".minecraft").join("runtime"),5,&mut candidate);
    }

    let mut seen = HashSet::new();
    let mut runtimes = Vec::new();
    for java in candidate {
        let name = java.to_string_lossy().to_ascii_lowercase();
        if !seen.insert(name) {continue;}
        if let Some(runtime) = run_java(&java) {runtimes.push(runtime);}
    }
    runtimes.sort_by(|a,b|b.major.cmp(&a.major).then_with(||a.path.cmp(&b.path)));
    runtimes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn java_version_parsing() {
        assert_eq!(java_major("openjdk version \"21.0.5\" 2024-10-15"),Some(21));
        assert_eq!(java_major("java version \"1.8.0_321\""),Some(8));
    }
}

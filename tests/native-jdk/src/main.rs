#[path = "../../../scripts/modrinth-fork/fre_rust_jdk.rs"]
mod jdk;

#[tokio::main]
async fn main() {
    let root = std::env::args_os().nth(1).expect("isolated data directory required");
    let root = std::path::PathBuf::from(root);
    let first = jdk::ensure(&root).await.expect("real JDK installation must succeed");
    assert!(first.join("bin/java.exe").is_file());
    assert!(first.join("bin/javac.exe").is_file());
    let second = jdk::ensure(&root).await.expect("cached JDK reuse must succeed");
    assert_eq!(first, second);
    println!("Real managed JDK 25 install/discovery and repeat reuse passed: {}", first.display());
}

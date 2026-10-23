fn main() {
    // WebView2's synchronous WebResourceRequested handling has nested native/Rust
    // frames that exceeded the Windows default 1 MiB main-thread stack in CI.
    // A WinDbg minidump (run 39985988909) identified 0xC00000FD after
    // ~640 KiB of stack use followed by another ~390 KiB allocation.
    // Reserve 8 MiB of *virtual* stack address space (not 8 MiB of RAM).
    // This is a crash mitigation, not an antivirus or security bypass.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-arg-bin=fre-launch=/STACK:8388608");
    }
    tauri_build::build();
}

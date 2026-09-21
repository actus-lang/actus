use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/runtime/mod.rs");

    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"));
    let archive = output_dir.join("libactus_runtime.a");
    let rustc = env::var_os("RUSTC").expect("Cargo must provide RUSTC");
    let status = Command::new(rustc)
        .args(["--edition", "2024", "--crate-type", "staticlib"])
        .arg("--crate-name")
        .arg("actus_runtime")
        .arg("src/runtime/mod.rs")
        .arg("-o")
        .arg(&archive)
        .status()
        .expect("failed to invoke rustc for the native runtime");
    assert!(status.success(), "native runtime compilation failed");
    println!("cargo:rustc-env=ACTUS_RUNTIME_ARCHIVE={}", archive.display());
}

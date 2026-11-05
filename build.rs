use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/runtime");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_231_PREFIX");
    println!("cargo:rustc-check-cfg=cfg(coverage)");
    println!("cargo:rustc-check-cfg=cfg(actus_test_faults)");

    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"));
    let archive = output_dir.join("libactus_runtime.a");
    let rustc = env::var_os("RUSTC").expect("Cargo must provide RUSTC");
    let mut command = Command::new(rustc);
    command
        .args(["--edition", "2024", "--crate-type", "staticlib"])
        .arg("--crate-name")
        .arg("actus_runtime")
        .arg("src/runtime/mod.rs");
    if env::var_os("CARGO_FEATURE_TEST_FAULT_INJECTION").is_some() {
        command.args(["--cfg", "actus_test_faults"]);
    }
    let status = command
        .arg("-o")
        .arg(&archive)
        .status()
        .expect("failed to invoke rustc for the native runtime");
    assert!(status.success(), "native runtime compilation failed");
    println!("cargo:rustc-env=ACTUS_RUNTIME_ARCHIVE={}", archive.display());

    if env::var_os("CARGO_FEATURE_LLVM").is_some() {
        build_llvm_target_options_shim();
    }
}

fn build_llvm_target_options_shim() {
    let llvm_config = env::var_os("DEP_LLVM_23_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("LLVM feature enabled but llvm-sys did not provide llvm-config"));
    println!("cargo:rustc-env=ACTUS_LLVM_CONFIG_PATH={}", llvm_config.display());
    let version = command_output(&llvm_config, "--version");
    let version = version.split_whitespace().next().unwrap_or_default();
    let mut components = version.split('.');
    let compatible = components.next() == Some("23") && components.next() == Some("1");
    assert!(
        compatible,
        "LLVM target-options shim requires LLVM 23.1, found `{version}` from `{}`",
        llvm_config.display()
    );
    let cxxflags = command_output(&llvm_config, "--cxxflags");
    let mut build = cc::Build::new();
    build.cpp(true).file("src/codegen/llvm/shim/target_options.cpp");
    for flag in cxxflags.split_whitespace() {
        build.flag(flag);
    }
    build.compile("actus_llvm_target_options");
    println!("cargo:rerun-if-changed=src/codegen/llvm/shim/target_options.cpp");
}

fn command_output(program: &std::path::Path, argument: &str) -> String {
    let output = Command::new(program)
        .arg(argument)
        .output()
        .unwrap_or_else(|error| panic!("failed to run `{}`: {error}", program.display()));
    assert!(
        output.status.success(),
        "`{} {argument}` failed: {}",
        program.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap_or_else(|error| {
        panic!("`{} {argument}` returned non-UTF-8 output: {error}", program.display())
    })
}

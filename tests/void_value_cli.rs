//! Bug #19: using the result of a verb that returns nothing is a clear
//! semantic error (E1141) from `check` and from `build` on every backend, and
//! never reaches code generation.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str =
    "verb greet() {\n    print(string: \"hi\");\n}\n\nverb main() {\n    erg x = greet();\n}\n";

fn project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-void-value-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"void-value\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write lockfile");
    fs::write(root.join("src/main.act"), SOURCE).expect("write source");
    root
}

fn rejected_stderr(root: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("run actus");
    assert!(!output.status.success(), "`actus {args:?}` must reject the program");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_clear_diagnostic(stderr: &str) {
    assert!(stderr.contains("src/main.act: error[E1141] at 6:13:"), "{stderr}");
    assert!(stderr.contains("`greet` returns nothing, so its call has no value"), "{stderr}");
    for internal in ["llvm", "lowering", "semantic-lowering", "cannot produce a value"] {
        assert!(!stderr.contains(internal), "internal text `{internal}` leaked: {stderr}");
    }
}

#[test]
fn check_reports_the_void_call_at_the_call() {
    let root = project("check");
    assert_clear_diagnostic(&rejected_stderr(&root, &["check", "src/main.act"]));
}

#[cfg(feature = "llvm")]
#[test]
fn build_on_the_llvm_backend_reports_the_void_call() {
    let root = project("llvm");
    let args = ["build", "src/main.act", "--backend", "llvm", "--emit", "exe", "-o", "out.bin"];
    assert_clear_diagnostic(&rejected_stderr(&root, &args));
}

#[cfg(feature = "cranelift")]
#[test]
fn build_on_the_cranelift_backend_reports_the_void_call() {
    let root = project("cranelift");
    let args =
        ["build", "src/main.act", "--backend", "cranelift", "--emit", "exe", "-o", "out.bin"];
    assert_clear_diagnostic(&rejected_stderr(&root, &args));
}

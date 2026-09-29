use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn missing_module_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-module-diagnostic-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create module fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"module-diagnostic\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write module fixture manifest");
    fs::write(root.join("src/main.act"), "import missing; verb main() -> Int { return 0; }\n")
        .expect("write missing module source");
    root
}

fn check_missing_module(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", "src/main.act"])
        .current_dir(root)
        .output()
        .expect("run missing module check")
}

#[test]
fn missing_module_diagnostics_are_deterministic_and_stable() {
    let root = missing_module_project();
    let first = check_missing_module(&root);
    let second = check_missing_module(&root);

    assert!(!first.status.success());
    assert_eq!(first.status.code(), second.status.code());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert!(String::from_utf8_lossy(&first.stderr).contains("error[E1101]"));
    assert!(String::from_utf8_lossy(&first.stderr).contains("missing its canonical facade"));
    fs::remove_dir_all(root).expect("remove module fixture");
}

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn strict_commands_report_mode_in_success_metadata() {
    let root = std::env::temp_dir().join(format!("actus-strict-metadata-{}", std::process::id()));
    write_strict_metadata_project(&root);
    for (command, arguments) in [
        ("check", vec!["check", "--strict"]),
        ("build", vec!["build", "--strict", "--emit", "obj"]),
        ("test", vec!["test", "--strict"]),
    ] {
        let output = run_strict_command(&root, &arguments);
        assert!(output.status.success(), "{command} failed: {:?}", output.stderr);
        assert!(String::from_utf8_lossy(&output.stdout).contains("(strict)"));
    }
    let _ = fs::remove_dir_all(root);
}

fn write_strict_metadata_project(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::create_dir_all(root.join("tests")).expect("create test directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"strict-metadata\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main() -> Int { return 0; }\n")
        .expect("write entry source");
    fs::write(root.join("tests/smoke.act"), "meta test\nverb smoke() -> Int { return 0; }\n")
        .expect("write test source");
}

fn run_strict_command(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .env("ACTUS_STRICT", "0")
        .current_dir(root)
        .output()
        .expect("run strict command")
}

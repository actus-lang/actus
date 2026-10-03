#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
#[test]
fn test_discovers_meta_test_verbs_and_reports_native_results() {
    let root = create_test_project(
        "runner",
        "smoke.act",
        "meta test\nverb smoke() -> Int { return 0; }\n",
    );
    let output = run_test_command(&root);
    assert!(output.status.success(), "test command failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 passed; 0 failed"));
    assert!(stdout.contains("exit code 0"));
    assert!(!stdout.contains("\x1b[32m"));
    assert!(!stdout.contains("\x1b[31m"));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn test_isolates_child_output_from_deterministic_runner_report() {
    let root = create_test_project(
        "runner-output",
        "noisy.act",
        "meta test\nverb noisy() -> Int { print(0); return 0; }\n",
    );
    let output = run_test_command(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert_eq!(stdout.lines().filter(|line| *line == "0").count(), 0, "stdout: {stdout}");
    assert!(stdout.contains("1 passed; 0 failed"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn lock_command_synchronizes_and_validates_dependency_state() {
    let root = create_test_project(
        "lock-command",
        "smoke.act",
        "meta test\nverb smoke() -> Int { return 0; }\n",
    );
    let update = run_in_project(&root, &["lock"]);
    assert!(update.success());
    let check = run_in_project(&root, &["lock", "--check"]);
    assert!(check.success());
    fs::write(root.join("Actus.lock"), "lockfile_version = 1\n").expect("corrupt lockfile");
    let stale = run_in_project(&root, &["lock", "--check"]);
    assert!(!stale.success());
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn test_reports_target_filtered_counts_deterministically() {
    let root = create_test_project(
        "runner-targets",
        "targets.act",
        "meta target(\"unix\") meta test\nverb unix_smoke() -> Int { return 0; }\nmeta target(\"windows\") meta test\nverb windows_smoke() -> Int { return 0; }\n",
    );
    let output = run_test_command(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_name = if cfg!(windows) { "windows_smoke" } else { "unix_smoke" };
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("running 1 tests (1 filtered)"), "stdout: {stdout}");
    assert!(stdout.contains(expected_name), "stdout: {stdout}");
    assert!(stdout.contains("1 passed; 0 failed"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn test_links_imported_module_objects_for_meta_tests() {
    let root = create_test_project(
        "runner-module",
        "module.act",
        "import math; meta test\nverb smoke() -> Int { return add(); }\n",
    );
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write module facade");
    fs::write(root.join("src/math/api.act"), "open verb add() -> Int { return 0; }\n")
        .expect("write module implementation");

    let output = run_test_command(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("smoke ... ok"), "stdout: {stdout}");
    assert!(stdout.contains("1 passed; 0 failed"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn test_compiles_meta_tests_against_their_complete_sibling_module() {
    let root = create_test_project(
        "runner-sibling-module",
        "placeholder.act",
        "meta test\nverb unused() -> Int { return 0; }\n",
    );
    fs::remove_file(root.join("tests/placeholder.act")).expect("remove placeholder test");
    fs::create_dir_all(root.join("src/aie")).expect("create module directory");
    fs::write(root.join("src/aie/aie.act"), "open fabric; open engine;\n")
        .expect("write module facade");
    fs::write(root.join("src/aie/fabric.act"), "open struct CorticalFabric { marker: Int, }\n")
        .expect("write sibling type");
    fs::write(
        root.join("src/aie/engine.act"),
        "meta test\nverb sees_sibling_type() -> Int {\n    erg fabric: CorticalFabric = CorticalFabric { marker: 0 };\n    return fabric.marker;\n}\n",
    )
    .expect("write test sibling");

    let output = run_test_command(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("sees_sibling_type ... ok"), "stdout: {stdout}");
    assert!(stdout.contains("1 passed; 0 failed"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn test_propagates_non_zero_meta_test_exit_status() {
    let root = create_test_project(
        "runner-failure",
        "failure.act",
        "meta test\nverb fails() -> Int { return 7; }\n",
    );
    let output = run_test_command(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("FAILED ("), "stdout: {stdout}");
    assert!(stdout.contains("exit code 7"), "stdout: {stdout}");
    assert!(stdout.contains("0 passed; 1 failed"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn fmt_check_reports_drift_then_accepts_canonical_output() {
    let root = std::env::temp_dir().join(format!("actus-fmt-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"formatter\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main()->Int{return 0;}\n").expect("write source");
    let before = run_in_project(&root, &["fmt", "--check"]);
    assert!(!before.success());
    assert!(run_in_project(&root, &["fmt"]).success());
    assert!(run_in_project(&root, &["fmt", "--check"]).success());
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
fn create_test_project(name: &str, file: &str, source: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("actus-{name}-{}", std::process::id()));
    fs::create_dir_all(root.join("tests")).expect("create test directory");
    fs::write(
        root.join("Actus.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n"),
    )
    .expect("write manifest");
    fs::write(root.join("tests").join(file), source).expect("write test source");
    root
}

#[cfg(unix)]
fn run_test_command(root: &std::path::Path) -> std::process::Output {
    run_command(root, &["test"])
}

#[cfg(unix)]
fn run_in_project(root: &std::path::Path, arguments: &[&str]) -> std::process::ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(root)
        .status()
        .expect("run project command")
}

#[cfg(unix)]
fn run_command(root: &std::path::Path, arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("run project command")
}

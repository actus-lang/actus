use std::fs;
use std::path::Path;
use std::process::Command;

use actus::cli::run_with_args;

#[test]
fn new_creates_an_actus_project_without_git_when_requested() {
    let root = std::env::temp_dir().join(format!("actus-new-{}", std::process::id()));
    let result = run_with_args(
        vec!["new".to_owned(), root.display().to_string(), "--no-git".to_owned()].into_iter(),
    );

    assert_eq!(result, 0);
    assert!(root.join("Arca.toml").is_file());
    assert!(root.join("src/main.act").is_file());
    assert_eq!(fs::read_to_string(root.join(".gitignore")).unwrap(), "/capsula/\n*.o\n*.bin\n");
    assert!(!root.join(".git").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn new_initializes_main_and_prints_next_steps() {
    let root = std::env::temp_dir().join(format!("actus-new-main-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["new", root.to_str().unwrap()])
        .output()
        .expect("run new command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("created `"));
    assert!(stdout.contains("Next steps:"));
    assert!(stdout.contains("actus check"));
    assert!(stdout.contains("actus build --release --emit exe"));
    assert_main_branch_or_parent_repository(&root);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn init_prints_next_steps_and_initializes_main() {
    let root = std::env::temp_dir().join(format!("actus-init-main-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create init directory");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["init"])
        .current_dir(&root)
        .output()
        .expect("run init command");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("actus run"));
    assert_main_branch_or_parent_repository(&root);
    let _ = fs::remove_dir_all(root);
}

fn assert_main_branch_or_parent_repository(root: &Path) {
    let parent_repository =
        root.ancestors().skip(1).any(|directory| directory.join(".git").exists());
    if parent_repository {
        assert!(!root.join(".git").exists());
        return;
    }
    let branch = Command::new("git")
        .args(["-C", root.to_str().unwrap(), "symbolic-ref", "--short", "HEAD"])
        .output()
        .expect("read initial branch");
    assert!(branch.status.success());
    assert_eq!(String::from_utf8_lossy(&branch.stdout).trim(), "main");
}

#[test]
fn init_creates_an_existing_directory_project_without_git() {
    let root = std::env::temp_dir().join(format!("actus-init-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create init directory");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["init", "--no-git"])
        .current_dir(&root)
        .status()
        .expect("run init command");

    assert!(status.success());
    assert!(root.join("Arca.toml").is_file());
    assert!(root.join("src/main.act").is_file());
    assert!(!root.join(".git").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_validates_source_without_emitting_code() {
    let input = std::env::temp_dir().join(format!("actus-check-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

    let result = run_with_args(vec!["check".to_owned(), input.display().to_string()].into_iter());

    assert_eq!(result, 0);
    assert!(!input.with_extension("o").exists());
    let _ = fs::remove_file(input);
}

#[cfg(unix)]
#[test]
fn build_discovers_the_manifest_entry_without_an_input_path() {
    let root = std::env::temp_dir().join(format!("actus-build-entry-{}", std::process::id()));
    let output = root.join("hello");
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::write(
        root.join("Arca.toml"),
        "[package]\nname = \"entry\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main() -> Int { return 42; }\n")
        .expect("write entry source");

    let result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&root)
        .output()
        .expect("run build command");

    assert!(
        result.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn build_discovers_a_parent_manifest_from_a_nested_directory() {
    let root = std::env::temp_dir().join(format!("actus-build-nested-{}", std::process::id()));
    let nested = root.join("src").join("nested");
    let output = nested.join("hello");
    fs::create_dir_all(&nested).expect("create nested source directory");
    fs::write(
        root.join("Arca.toml"),
        "[package]\nname = \"nested\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main() -> Int { return 43; }\n")
        .expect("write entry source");

    let result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&nested)
        .output()
        .expect("run nested build command");

    assert!(
        result.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(43));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn run_accepts_release_profile_and_forwards_arguments() {
    let input = std::env::temp_dir().join(format!("actus-run-options-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

    let result = run_with_args(
        vec![
            "run".to_owned(),
            input.display().to_string(),
            "--release".to_owned(),
            "--".to_owned(),
            "argument".to_owned(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 42);
    let _ = fs::remove_file(input);
}

#[cfg(unix)]
#[test]
fn test_discovers_meta_test_verbs_and_reports_native_results() {
    let root = std::env::temp_dir().join(format!("actus-test-runner-{}", std::process::id()));
    fs::create_dir_all(root.join("tests")).expect("create test directory");
    fs::write(
        root.join("Arca.toml"),
        "[package]\nname = \"runner\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("tests/smoke.act"), "meta test\nverb smoke() -> Int { return 0; }\n")
        .expect("write meta test");

    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("test")
        .current_dir(&root)
        .output()
        .expect("run test command");

    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 passed; 0 failed"));
    assert!(stdout.contains("exit code 0"));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn fmt_check_reports_drift_then_accepts_canonical_output() {
    let root = std::env::temp_dir().join(format!("actus-fmt-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::write(
        root.join("Arca.toml"),
        "[package]\nname = \"formatter\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main()->Int{return 0;}\n").expect("write source");

    let before = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["fmt", "--check"])
        .current_dir(&root)
        .status()
        .expect("run formatter check");
    assert!(!before.success());
    let format = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("fmt")
        .current_dir(&root)
        .status()
        .expect("run formatter");
    assert!(format.success());
    let after = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["fmt", "--check"])
        .current_dir(&root)
        .status()
        .expect("run formatter check after formatting");
    assert!(after.success());
    let _ = fs::remove_dir_all(root);
}

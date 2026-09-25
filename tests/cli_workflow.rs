use std::fs;
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

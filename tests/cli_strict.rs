use std::fs;

use actus::cli::run_with_args;

#[test]
fn strict_check_accepts_a_semantically_valid_source() {
    let input = std::env::temp_dir().join(format!("actus-strict-check-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

    let result = run_with_args(
        vec!["check".to_owned(), input.display().to_string(), "--strict".to_owned()].into_iter(),
    );

    assert_eq!(result, 0);
    let _ = fs::remove_file(input);
}

#[test]
fn strict_build_rejects_semantic_errors_before_emitting_output() {
    let root = std::env::temp_dir().join(format!("actus-strict-build-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("o");
    fs::write(&input, "verb main() -> Int { return missing; }\n").expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--strict".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 1);
    assert!(!output.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn strict_options_are_rejected_when_repeated() {
    assert_eq!(
        run_with_args(
            vec!["check".to_owned(), "--strict".to_owned(), "--strict".to_owned()].into_iter(),
        ),
        2
    );
    assert_eq!(
        run_with_args(
            vec!["build".to_owned(), "--strict".to_owned(), "--strict".to_owned()].into_iter(),
        ),
        2
    );
    assert_eq!(
        run_with_args(
            vec!["test".to_owned(), "--strict".to_owned(), "--strict".to_owned()].into_iter(),
        ),
        2
    );
}

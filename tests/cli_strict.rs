use std::fs;
use std::process::Command;

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
fn strict_build_fails_closed_before_codegen_and_cleans_failed_outputs() {
    let root = std::env::temp_dir().join(format!("actus-strict-frontend-{}", std::process::id()));
    let cases = [
        ("lexical", "verb main() -> Int { return @; }\n"),
        ("parser", "verb main() -> Int { return 0 }\n"),
    ];

    for (name, source) in cases {
        let input = root.with_extension(format!("{name}.act"));
        let output = root.with_extension(format!("{name}.o"));
        fs::write(&input, source).expect("write invalid source");
        for _ in 0..2 {
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
            assert!(!output.with_extension("actmeta").exists());
        }
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[test]
fn strict_build_rejects_declaration_contract_violations_without_artifact() {
    let root =
        std::env::temp_dir().join(format!("actus-strict-declarations-{}", std::process::id()));
    let cases = [
        ("duplicate", "verb main() -> Int { return 0; } verb main() -> Int { return 0; }\n"),
        ("empty_enum", "enum Nothing { } verb main() -> Int { return 0; }\n"),
        (
            "incomplete",
            "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; verb flush(abs self: File); } perform Writer for File { verb write(abs self: File) -> Int { return 0; } } verb main() -> Int { return 0; }\n",
        ),
    ];

    for (name, source) in cases {
        let input = root.with_extension(format!("{name}.act"));
        let output = root.with_extension(format!("{name}.o"));
        fs::write(&input, source).expect("write declaration violation");
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

#[test]
fn strict_commands_reject_unknown_forms_and_extra_arguments() {
    for form in ["--strict=true", "--strict=false", "--strict="] {
        for command in ["check", "build", "test"] {
            assert_eq!(
                run_with_args(vec![command.to_owned(), form.to_owned()].into_iter()),
                2,
                "{command} must reject {form}"
            );
        }
    }
    assert_eq!(
        run_with_args(
            vec![
                "check".to_owned(),
                "--strict".to_owned(),
                "first.act".to_owned(),
                "extra.act".to_owned()
            ]
            .into_iter(),
        ),
        2
    );
    assert_eq!(
        run_with_args(
            vec!["test".to_owned(), "--strict".to_owned(), "unexpected.act".to_owned()].into_iter(),
        ),
        2
    );
}

#[test]
fn strict_commands_reject_misplaced_and_conflicting_forms() {
    assert_eq!(
        run_with_args(
            vec![
                "check".to_owned(),
                "input.act".to_owned(),
                "--strict".to_owned(),
                "extra.act".to_owned(),
            ]
            .into_iter(),
        ),
        2
    );
    assert_eq!(
        run_with_args(
            vec!["build".to_owned(), "--strict".to_owned(), "--strict=true".to_owned()].into_iter(),
        ),
        2
    );
    assert_eq!(
        run_with_args(
            vec!["test".to_owned(), "unexpected.act".to_owned(), "--strict".to_owned()].into_iter(),
        ),
        2
    );
}

#[test]
fn strict_check_rejects_malformed_configuration_before_source_compilation() {
    let root = std::env::temp_dir().join(format!("actus-strict-malformed-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::write(root.join("Actus.toml"), "[package\ninvalid").expect("write malformed manifest");
    let input = root.join("src/main.act");
    fs::write(&input, "verb main() -> Int { return ; }\n").expect("write invalid source");

    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().unwrap(), "--strict"])
        .output()
        .expect("run strict check");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("E1800"));
    assert!(!stderr.contains("E0003"));
    let _ = fs::remove_dir_all(root);
}

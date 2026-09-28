use std::fs;
use std::path::{Path, PathBuf};

use actus::conformance::{
    SourceLimitPolicy, fixture_tree_diagnostics, inspect_source_tree, source_limit_diagnostics,
    source_paths, validate_source_exception_manifest,
};
use actus::diagnostics::{DiagnosticPhase, DiagnosticSeverity};

fn lines(count: usize) -> String {
    (0..count).map(|index| format!("line_{index}\n")).collect()
}

fn policy() -> SourceLimitPolicy {
    SourceLimitPolicy::default()
}

#[test]
fn preferred_file_limit_reports_decomposition_evidence() {
    let diagnostics = source_limit_diagnostics(Path::new("src/example.rs"), &lines(301), policy());
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "E1850");
    assert_eq!(diagnostics[0].source_path(), Some("src/example.rs"));
    assert_eq!(diagnostics[0].phase(), DiagnosticPhase::Conformance);
}

#[test]
fn strict_file_thresholds_escalate_and_stop_at_one_violation() {
    let split = source_limit_diagnostics(Path::new("src/example.rs"), &lines(400), policy());
    assert_eq!(split[0].code(), "E1851");
    assert_eq!(split[0].severity(), DiagnosticSeverity::Error);

    let hard = source_limit_diagnostics(Path::new("src/example.rs"), &lines(501), policy());
    assert_eq!(hard[0].code(), "E1852");
    assert_eq!(hard[0].severity(), DiagnosticSeverity::Error);
}

#[test]
fn rust_functions_are_measured_without_counting_comment_text() {
    let mut source = String::from("/* fn fake() {\nstill comment } */\nfn real() {\n");
    source.push_str(&lines(40));
    source.push_str("}\n");
    let diagnostics = source_limit_diagnostics(Path::new("src/example.rs"), &source, policy());
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "E1854"));
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.severity() == DiagnosticSeverity::Error)
    );
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.message().contains("real")));
}

#[test]
fn actus_verbs_use_the_same_function_contract() {
    let mut source = String::from("verb run() {\n");
    source.push_str(&lines(60));
    source.push_str("}\n");
    let diagnostics =
        source_limit_diagnostics(Path::new("library/std/src/run.act"), &source, policy());
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "E1855"));
}

#[test]
fn short_sources_have_no_limit_diagnostics() {
    let source = "verb main() { return; }\n";
    assert!(source_limit_diagnostics(Path::new("main.act"), source, policy()).is_empty());
}

#[test]
fn source_local_limit_suppression_is_rejected() {
    let source = "// actus: allow(source-limit)\nverb main() { return; }\n";
    let diagnostics = source_limit_diagnostics(Path::new("main.act"), source, policy());
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "E1856"));
}

#[test]
fn rust_logic_in_library_fixture_tree_is_rejected() {
    let root = std::env::temp_dir().join(format!("actus-fixture-policy-{}", std::process::id()));
    let fixture = root.join("tests/library/embedded.rs");
    fs::create_dir_all(fixture.parent().expect("fixture parent should exist"))
        .expect("fixture directory should be created");
    fs::write(&fixture, "fn hidden_test() {}\n").expect("fixture should be written");
    let diagnostics = fixture_tree_diagnostics(&root).expect("fixture tree should be scanned");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "E1834");
    fs::remove_dir_all(root).expect("fixture tree should be removed");
}

#[test]
fn source_exception_manifest_requires_review_fields() {
    let valid = "version = 1\n[[exceptions]]\npath = \"tests/fixture.act\"\ncategory = \"tabular\"\nowner = \"compiler-team\"\nscope = \"fixture only\"\nreason = \"generated compatibility table\"\nreplacement_plan = \"split before Phase 19\"\n";
    assert!(validate_source_exception_manifest(valid).is_ok());
    let invalid = "version = 1\n[[exceptions]]\npath = \"tests/fixture.act\"\ncategory = \"manual\"\nowner = \"team\"\nscope = \"fixture\"\nreason = \"reason\"\nreplacement_plan = \"plan\"\n";
    assert!(validate_source_exception_manifest(invalid).is_err());
}

#[test]
fn repository_sources_have_no_hard_limit_violations() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest.join("src");
    let library_root = manifest.join("library/std/src");
    let diagnostics = inspect_source_tree(&[source_root.as_path(), library_root.as_path()])
        .expect("repository source tree should be readable");
    let hard_violations = diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.code(), "E1852" | "E1855"))
        .map(|diagnostic| {
            format!("{}: {}", diagnostic.source_path().unwrap_or("<unknown>"), diagnostic.message())
        })
        .collect::<Vec<_>>();
    assert!(
        hard_violations.is_empty(),
        "hard source-limit diagnostics:\n{}",
        hard_violations.join("\n")
    );
}

#[test]
fn repository_source_paths_are_sorted_and_include_supported_roots() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let roots = [manifest.join("src"), manifest.join("tests")];
    let root_refs = roots.iter().map(PathBuf::as_path).collect::<Vec<_>>();
    let paths = source_paths(&root_refs).expect("repository source paths should be readable");
    assert!(paths.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(paths.iter().any(|path| path.ends_with("src/lib.rs")));
    assert!(paths.iter().any(|path| path.ends_with("tests/source_limits.rs")));
}

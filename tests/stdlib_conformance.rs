use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use actus::ast::{MetaAttribute, TestFault, TopLevelDecl};
use actus::conformance::{inspect_source, source_paths};
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn standard_library_sources_pass_strict_source_limits() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let files = source_paths(&[&root]).expect("standard-library sources should be discoverable");
    assert!(!files.is_empty(), "standard-library source inventory must not be empty");
    let diagnostics = files.iter().flat_map(diagnostics_for).collect::<Vec<_>>();
    assert!(diagnostics.is_empty(), "standard-library strict diagnostics: {diagnostics:?}");
}

#[test]
fn library_fixtures_are_actus_meta_test_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std/tests");
    let files = source_paths(&[&root]).expect("library fixture sources should be discoverable");
    assert!(!files.is_empty(), "library fixture inventory must not be empty");
    for path in files {
        assert_eq!(path.extension().and_then(|extension| extension.to_str()), Some("act"));
        let source = fs::read_to_string(&path).expect("library fixture should be readable");
        assert!(source.contains("meta test"), "fixture has no meta test: {}", path.display());
    }
}

#[test]
fn every_registered_fault_has_a_library_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std/tests");
    let files = source_paths(&[&root]).expect("library fixture sources should be discoverable");
    let mut covered = HashSet::new();
    for path in files {
        let source = fs::read_to_string(&path).expect("library fixture should be readable");
        let (tokens, errors) = scan(&source);
        assert!(errors.is_empty(), "fixture should lex: {}", path.display());
        let program = parse(tokens).expect("library fixture should parse");
        for declaration in program.declarations {
            let TopLevelDecl::Verb(verb) = declaration else { continue };
            for attribute in verb.metadata {
                if let MetaAttribute::Fault(fault) = attribute {
                    covered.insert(fault.name());
                }
            }
        }
    }
    for fault in TestFault::ALL {
        assert!(covered.contains(fault.name()), "missing strict fixture for `{}`", fault.name());
    }
}

fn diagnostics_for(path: &PathBuf) -> Vec<String> {
    let source = fs::read_to_string(path).expect("standard-library source should be readable");
    inspect_source(path, &source)
        .into_iter()
        .map(|diagnostic| format!("{}: {}", path.display(), diagnostic.message()))
        .collect()
}

/// The standard library's tests are the project tests of `library/std`
/// (ADR-0121): the runner must discover them there, every fixture file in one
/// namespace.
#[test]
fn library_fixtures_are_discovered_by_the_test_runner_in_library_std() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["test", "--list"])
        .current_dir(&root)
        .output()
        .expect("run actus test --list in library/std");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("tests::ka_decode_round_trips_mapped_bytes"), "stdout: {stdout}");
    // The fault fixtures are `meta target("unix")`: listed on Unix hosts and
    // filtered out elsewhere.
    let fault_fixture_listed = stdout.contains("tests::injected_short_write_is_rejected");
    assert_eq!(fault_fixture_listed, cfg!(unix), "stdout: {stdout}");
}

/// Tests are not part of the shipped standard library: the runtime checksum
/// recorded in the repository `Actus.lock` is the content checksum of
/// `library/std`, and `library/std/tests` must not change it (ADR-0121).
#[test]
fn standard_library_tests_do_not_change_the_runtime_checksum() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(root.join("library/std/tests").is_dir(), "std tests directory must exist");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["lock", "--check"])
        .current_dir(root)
        .output()
        .expect("run actus lock --check");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
}

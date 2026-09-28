use std::fs;
use std::path::{Path, PathBuf};

use actus::conformance::{inspect_source, source_paths};

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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/library");
    let files = source_paths(&[&root]).expect("library fixture sources should be discoverable");
    assert!(!files.is_empty(), "library fixture inventory must not be empty");
    for path in files {
        assert_eq!(path.extension().and_then(|extension| extension.to_str()), Some("act"));
        let source = fs::read_to_string(&path).expect("library fixture should be readable");
        assert!(source.contains("meta test"), "fixture has no meta test: {}", path.display());
    }
}

fn diagnostics_for(path: &PathBuf) -> Vec<String> {
    let source = fs::read_to_string(path).expect("standard-library source should be readable");
    inspect_source(path, &source)
        .into_iter()
        .map(|diagnostic| format!("{}: {}", path.display(), diagnostic.message()))
        .collect()
}

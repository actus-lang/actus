use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_FILENAMES: &[&str] = &[
    "utils.rs",
    "helpers.rs",
    "common.rs",
    "misc.rs",
    "utils.act",
    "helpers.act",
    "common.act",
    "misc.act",
];

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn child_directories(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .expect("architecture fixture directory should be readable")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            path.is_dir().then_some(path)
        })
        .collect()
}

fn has_direct_act_file(directory: &Path) -> bool {
    fs::read_dir(directory)
        .expect("Actus source directory should be readable")
        .filter_map(Result::ok)
        .any(|entry| {
            entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "act")
        })
}

fn assert_canonical_facades(directory: &Path) {
    for child in child_directories(directory) {
        if has_direct_act_file(&child) {
            let name = child.file_name().expect("module directory name").to_string_lossy();
            assert!(
                child.join(format!("{name}.act")).is_file(),
                "module directory {} must expose {}.act",
                child.display(),
                name
            );
        }
        assert_canonical_facades(&child);
    }
}

fn assert_no_forbidden_filenames(directory: &Path) {
    for entry in fs::read_dir(directory).expect("source directory should be readable") {
        let path = entry.expect("source directory entry").path();
        if path.is_dir() {
            assert_no_forbidden_filenames(&path);
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        assert!(
            !FORBIDDEN_FILENAMES.contains(&name.as_ref()),
            "forbidden generic file: {}",
            path.display()
        );
    }
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).expect("Rust source directory should be readable") {
        let path = entry.expect("Rust source directory entry").path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

fn assert_layer_excludes(directory: &str, forbidden: &[&str]) {
    let root = repository_path(directory);
    for path in rust_sources(&root) {
        let source = fs::read_to_string(&path).expect("Rust source should be readable");
        for token in forbidden {
            assert!(!source.contains(token), "{} contains forbidden `{token}`", path.display());
        }
    }
}

#[test]
fn standard_library_directories_have_canonical_facades() {
    assert_canonical_facades(&repository_path("library/std/src"));
}

#[test]
fn generic_implementation_filenames_are_rejected() {
    assert_no_forbidden_filenames(&repository_path("src"));
    assert_no_forbidden_filenames(&repository_path("library/std/src"));
}

#[test]
fn compiler_layers_do_not_import_reverse_pipeline_or_backend_logic() {
    assert_layer_excludes(
        "src/lexer",
        &["crate::parser", "crate::ast", "crate::semantic", "crate::codegen"],
    );
    assert_layer_excludes(
        "src/parser",
        &["crate::semantic", "crate::codegen", "OwnershipState", "BorrowRecord", "SemanticError"],
    );
    assert_layer_excludes("src/ast", &["crate::codegen", "cranelift", "NativeType"]);
    assert_layer_excludes(
        "src/semantic",
        &["crate::codegen", "cranelift", "render_diagnostic", "println!", "eprintln!"],
    );
    assert_layer_excludes(
        "src/codegen",
        &["SemanticErrorKind", "SemanticError", "Analyzer", "render_diagnostic"],
    );
}

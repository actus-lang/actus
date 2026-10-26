use std::path::PathBuf;

use actus::diagnostics::module_diagnostic;
use actus::lexer::scan;
use actus::modules::{ModuleError, ModuleResolver, analyze_module, analyze_with_imports};
use actus::parser::parse;

fn std_resolver() -> ModuleResolver {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    ModuleResolver::new(root)
}

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

#[test]
fn std_io_internal_scope_still_analyzes_after_bridge_privacy_migration() {
    analyze_module(&std_resolver(), "io").expect("std::io internal scope should remain valid");
}

#[test]
fn std_io_raw_bridge_is_not_importable_through_the_public_facade() {
    let source =
        parse_source("import io; verb main() -> Int { return actus_print_int(value: 1); }");
    let error = analyze_with_imports(&source, &std_resolver())
        .expect_err("raw std::io bridges must remain private to the module");
    assert!(
        matches!(error, ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "actus_print_int")
    );
    assert_eq!(module_diagnostic(&error).code(), "E1109");
}

#[test]
fn std_fs_raw_bridge_is_not_importable_through_the_public_facade() {
    let source = parse_source(
        "import fs; verb main() -> Int { return actus_file_remove_path(path: missing); }",
    );
    let error = analyze_with_imports(&source, &std_resolver())
        .expect_err("raw std::fs bridges must remain private to the module");
    assert!(
        matches!(error, ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "actus_file_remove_path")
    );
    assert_eq!(module_diagnostic(&error).code(), "E1109");
}

#[test]
fn std_fs_private_child_must_be_reached_through_the_canonical_facade() {
    let error = std_resolver()
        .resolve("fs::file")
        .expect_err("std::fs::file must not bypass the std::fs facade");
    assert!(matches!(
        error,
        actus::modules::ModuleResolutionError::BypassesFacade { parent, .. }
            if parent == "fs"
    ));
}

#[test]
fn std_directory_moved_owner_is_rejected_before_codegen() {
    let source = parse_source(
        "import { open fs }; verb main() -> Int { erg directory: Directory = Directory { handle: 1, }; drop(directory); next_dir_entry(directory: ins directory); return 0; }",
    );
    analyze_with_imports(&source, &std_resolver())
        .expect_err("a dropped directory owner must not reach directory code generation");
}

#[test]
fn std_path_raw_bridge_is_not_importable_through_the_public_facade() {
    let source = parse_source(
        "import path; verb main() -> Int { return actus_path_is_absolute(path: missing); }",
    );
    let error = analyze_with_imports(&source, &std_resolver())
        .expect_err("raw std::path bridges must remain private to the module");
    assert!(
        matches!(error, ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "actus_path_is_absolute")
    );
    assert_eq!(module_diagnostic(&error).code(), "E1109");
}

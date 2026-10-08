use std::path::PathBuf;

use actus::lexer::scan;
use actus::modules::{ModuleResolver, analyze_module, analyze_with_imports, exports_module};
use actus::parser::parse;

fn std_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src")
}

#[test]
fn region_facade_exports_checked_public_operations() {
    let resolver = ModuleResolver::new(std_root());
    let exports = exports_module(&resolver, "region").expect("std region facade should resolve");
    assert!(exports.contains("verb", "region_open"));
    assert!(exports.contains("verb", "region_read"));
    assert!(exports.contains("verb", "region_read_range"));
    assert!(exports.contains("verb", "region_write"));
    assert!(exports.contains("verb", "region_write_range"));
    assert!(exports.contains("verb", "region_publish"));
    assert!(exports.contains("verb", "region_cancel"));
    assert!(exports.contains("verb", "region_close"));
    assert!(exports.contains("verb", "region_remap"));
    assert!(exports.contains("verb", "region_logical_length"));
    assert!(exports.contains("verb", "region_window_start"));
    assert!(exports.contains("verb", "region_window_count"));
    assert!(exports.contains("verb", "region_generation"));
    assert!(exports.contains("verb", "region_dirty"));
    assert!(exports.contains("enum", "RegionError"));
    analyze_module(&resolver, "region").expect("std region declarations should be valid");
}

#[test]
fn region_private_bridges_are_not_exported() {
    let source = "import region; verb main() -> Int { return actus_region_close[ u32 ](region: ins value); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "fixture should lex without errors: {errors:?}");
    let program = parse(tokens).expect("private bridge fixture should parse");
    assert!(
        analyze_with_imports(&program, &ModuleResolver::new(std_root())).is_err(),
        "private region bridge must remain inaccessible"
    );
}

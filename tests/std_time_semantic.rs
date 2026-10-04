use std::path::PathBuf;

use actus::lexer::scan;
use actus::modules::{ModuleResolver, analyze_module, analyze_with_imports, exports_module};
use actus::parser::parse;

fn std_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src")
}

#[test]
fn std_time_facade_exports_documented_monotonic_api() {
    let resolver = ModuleResolver::new(std_root());
    let exports = exports_module(&resolver, "time").expect("std time facade should resolve");
    assert!(exports.contains("verb", "monotonic_nanos"));
    assert!(exports.contains("struct", "Instant"));
    assert!(exports.contains("struct", "Duration"));
    assert!(exports.contains("struct", "Deadline"));
    assert!(exports.contains("enum", "TimeError"));
    assert!(exports.contains("verb", "now"));
    assert!(exports.contains("verb", "duration_since"));
    assert!(exports.contains("verb", "deadline_from_now"));
    assert!(exports.contains("verb", "sleep"));
    analyze_module(&resolver, "time").expect("std time declarations should be semantically valid");
}

#[test]
fn hosted_program_can_import_and_call_monotonic_api() {
    let source = "import time; verb measure() -> u64 { return monotonic_nanos(); } verb main() -> Int { erg value: u64 = measure(); return value as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "fixture should lex without errors: {errors:?}");
    let program = parse(tokens).expect("hosted time fixture should parse");
    analyze_with_imports(&program, &ModuleResolver::new(std_root()))
        .expect("hosted time fixture should be semantically valid");
}

#[test]
fn private_native_bridge_is_not_exported_by_time_facade() {
    let source = "import time; verb main() -> Int { return actus_monotonic_nanos() as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "fixture should lex without errors: {errors:?}");
    let program = parse(tokens).expect("private bridge fixture should parse");
    assert!(
        analyze_with_imports(&program, &ModuleResolver::new(std_root())).is_err(),
        "private native bridge must remain inaccessible"
    );
}

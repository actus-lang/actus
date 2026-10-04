use std::path::PathBuf;

use actus::lexer::scan;
use actus::modules::{ModuleResolver, analyze_with_imports, exports_module};
use actus::parser::parse;

fn std_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src")
}

#[test]
fn string_facade_exports_borrowed_byte_api() {
    let resolver = ModuleResolver::new(std_root());
    let exports = exports_module(&resolver, "string").expect("string facade should resolve");
    assert!(exports.contains("verb", "string_length"));
    assert!(exports.contains("verb", "string_byte_at"));
    assert!(exports.contains("verb", "utf8_from_buffer"));
    assert!(exports.contains("verb", "utf8_byte_at"));
    assert!(exports.contains("struct", "Utf8Buffer"));
    assert!(exports.contains("enum", "StringError"));
    analyze_with_imports(
        &parse(scan("import string; verb main() -> Int { return 0; }").0).expect("parse"),
        &resolver,
    )
    .expect("string facade should be semantically valid");
}

#[test]
fn string_native_bridge_is_private() {
    let source = "import string; verb main() -> Int { return actus_string_length(text: \"x\"); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("parse");
    assert!(analyze_with_imports(&program, &ModuleResolver::new(std_root())).is_err());
}

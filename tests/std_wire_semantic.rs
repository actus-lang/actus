use std::path::PathBuf;

use actus::diagnostics::module_diagnostic;
use actus::lexer::scan;
use actus::modules::{
    ModuleError, ModuleResolver, analyze_module, analyze_with_imports, exports_module,
};
use actus::parser::parse;

fn std_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src")
}

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "fixture should lex without errors: {errors:?}");
    parse(tokens).expect("fixture should parse")
}

#[test]
fn wire_facade_exports_frame_error_and_checksum_contracts() {
    let resolver = ModuleResolver::new(std_root());
    let exports = exports_module(&resolver, "wire").expect("std wire facade should resolve");
    assert!(exports.contains("pack", "WireHeader"));
    assert!(exports.contains("enum", "WireError"));
    assert!(exports.contains("verb", "wire_crc16_ccitt"));
    assert!(exports.contains("verb", "wire_header_encode"));
    assert!(exports.contains("verb", "wire_header_decode"));
    assert!(exports.contains("verb", "wire_frame_encode"));
    assert!(exports.contains("verb", "wire_frame_decode"));
    assert!(exports.contains("enum", "WireParserStatus"));
    assert!(exports.contains("verb", "wire_parser_empty"));
    assert!(exports.contains("verb", "wire_parser_feed"));
    assert!(exports.contains("const", "WIRE_MAX_PAYLOAD_BYTES"));
    analyze_module(&resolver, "wire").expect("std wire declarations should be valid");
}

#[test]
fn wire_checksum_update_helper_is_not_public() {
    let source = parse_source(
        "import wire; verb main() -> Int { return wire_crc16_update(checksum: erg 0u16, byte: erg 0u8) as Int; }",
    );
    let error = analyze_with_imports(&source, &ModuleResolver::new(std_root()))
        .expect_err("wire checksum helper must remain private to the module");
    assert!(
        matches!(error, ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "wire_crc16_update")
    );
    assert_eq!(module_diagnostic(&error).code(), "E1109");
}

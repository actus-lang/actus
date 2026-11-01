use super::{parse_error, parse_source};
use actus::ast::{MetaAttribute, Repr, TopLevelDecl};
use actus::diagnostics::parse_diagnostic;
use actus::parser::ParseErrorCode;

#[test]
fn parses_bare_c_representation_on_structs_and_enums() {
    let program = parse_source(
        "meta repr(C) struct Point { x: i32, y: i32, } meta repr(C) enum Axis { X, Y, }",
    );
    let TopLevelDecl::Struct(structure) = &program.declarations[0] else {
        panic!("expected struct")
    };
    assert_eq!(structure.metadata, vec![MetaAttribute::Repr(Repr::C)]);
    let TopLevelDecl::Enum(enumeration) = &program.declarations[1] else { panic!("expected enum") };
    assert_eq!(enumeration.metadata, vec![MetaAttribute::Repr(Repr::C)]);
}

#[test]
fn rejects_string_representation_names() {
    let error = parse_error("meta repr(\"C\") struct Point { x: i32, }");
    assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
    assert_eq!(parse_diagnostic(&error).code(), "E0003");
}

#[test]
fn rejects_unknown_representation_names() {
    let error = parse_error("meta repr(packed) struct Point { x: i32, }");
    assert_eq!(error.code, ParseErrorCode::UnsupportedRepresentation);
    assert_eq!(parse_diagnostic(&error).code(), "E0022");
}

#[test]
fn rejects_duplicate_representation_metadata() {
    let error = parse_error("meta repr(C) meta repr(C) struct Point { x: i32, }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn rejects_representation_on_non_type_declarations() {
    let error = parse_error("meta repr(C) verb main() -> Int { return 0; }");
    assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed);
}

#[test]
fn rejects_representation_on_packs() {
    let error = parse_error(
        "meta repr(C) pack Register { erg storage: u32; layout little; fields { erg value: u32 at 0; } }",
    );
    assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed);
}

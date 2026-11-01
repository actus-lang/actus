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
fn parses_grouped_and_separate_representation_modifiers() {
    let program = parse_source(
        "meta repr(C, packed) struct Grouped { value: i32, } meta repr(C) meta repr(packed) struct Separate { value: i32, }",
    );
    let TopLevelDecl::Struct(grouped) = &program.declarations[0] else { panic!("expected struct") };
    assert_eq!(grouped.metadata, vec![MetaAttribute::Repr(Repr::new(true, true))]);
    let TopLevelDecl::Struct(separate) = &program.declarations[1] else {
        panic!("expected struct")
    };
    assert_eq!(separate.repr(), Some(Repr::new(true, true)));
}

#[test]
fn parses_transparent_representation() {
    let program = parse_source("meta repr(transparent) struct UserId { value: u64, }");
    let TopLevelDecl::Struct(structure) = &program.declarations[0] else {
        panic!("expected struct")
    };
    assert_eq!(structure.metadata, vec![MetaAttribute::Repr(Repr::TRANSPARENT)]);
}

#[test]
fn parses_explicit_alignment_in_grouped_and_separate_forms() {
    let program = parse_source(
        "meta repr(C, align(8u32)) struct Grouped { value: u8, } meta repr(C) meta repr(align(16u32)) struct Separate { value: u8, }",
    );
    let TopLevelDecl::Struct(grouped) = &program.declarations[0] else {
        panic!("expected grouped struct")
    };
    assert_eq!(grouped.repr(), Some(Repr::C.with_alignment(8)));
    let TopLevelDecl::Struct(separate) = &program.declarations[1] else {
        panic!("expected separately annotated struct")
    };
    assert_eq!(separate.repr(), Some(Repr::C.with_alignment(16)));
}

#[test]
fn rejects_string_representation_names() {
    let error = parse_error("meta repr(\"C\") struct Point { x: i32, }");
    assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
    assert_eq!(parse_diagnostic(&error).code(), "E0003");
}

#[test]
fn rejects_duplicate_representation_modifiers() {
    let error = parse_error("meta repr(C, C) struct Point { x: i32, }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
    assert_eq!(parse_diagnostic(&error).code(), "E0013");
}

#[test]
fn rejects_duplicate_transparent_representation_modifiers() {
    let error = parse_error("meta repr(transparent, transparent) struct UserId { value: u64, }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn rejects_duplicate_explicit_alignment_modifiers() {
    let error = parse_error("meta repr(align(8u32), align(16u32)) struct Aligned { value: u8, }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
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

use super::{parse_error, parse_source};
use actus::ast::{MetaAttribute, TopLevelDecl};
use actus::parser::ParseErrorCode;

#[test]
fn parses_copy_on_structs_and_enums() {
    let program = parse_source(
        "meta copy struct Header { kind: u8, } meta copy enum Mode { Idle, Run(u8), }",
    );
    let TopLevelDecl::Struct(structure) = &program.declarations[0] else {
        panic!("expected struct")
    };
    assert!(structure.is_copy());
    assert!(structure.metadata.contains(&MetaAttribute::Copy));
    let TopLevelDecl::Enum(enumeration) = &program.declarations[1] else { panic!("expected enum") };
    assert!(enumeration.is_copy());
}

#[test]
fn copy_combines_with_a_representation_contract() {
    let program = parse_source("meta repr(C) meta copy struct Header { kind: u8, }");
    let TopLevelDecl::Struct(structure) = &program.declarations[0] else {
        panic!("expected struct")
    };
    assert!(structure.is_copy());
    assert!(structure.repr().is_some_and(|repr| repr.is_c()));
}

#[test]
fn rejects_copy_on_a_verb() {
    let error = parse_error("meta copy verb run() { }");
    assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed);
}

#[test]
fn rejects_copy_on_a_field() {
    // Metadata is not part of a field list, so a field never accepts it.
    let error = parse_error("struct Header { meta copy kind: u8, }");
    assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
}

#[test]
fn rejects_duplicate_copy_metadata() {
    let error = parse_error("meta copy meta copy struct Header { kind: u8, }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn parses_copy_on_packs() {
    let program = parse_source(
        "meta copy pack Cell { erg storage: u8; layout little; fields { erg kind: u8 at 0; } } \
         meta copy open pack Line { erg storage: Array[u8, 2]; layout big; fields { erg word: u16 at 0; } } \
         pack Plain { erg storage: u8; layout little; fields { erg kind: u8 at 0; } }",
    );
    let packs: Vec<_> = program
        .declarations
        .iter()
        .map(|declaration| match declaration {
            TopLevelDecl::Pack(pack) => pack,
            other => panic!("expected pack, found {other:?}"),
        })
        .collect();
    assert!(packs[0].is_copy() && !packs[0].is_open);
    assert!(packs[0].metadata.contains(&MetaAttribute::Copy));
    assert!(packs[1].is_copy() && packs[1].is_open);
    assert!(!packs[2].is_copy());
}

#[test]
fn rejects_every_other_attribute_on_a_pack() {
    for attribute in ["repr(C)", "volatile", "zero_on_drop", "test", "deterministic"] {
        let error = parse_error(&format!(
            "meta {attribute} pack Cell {{ erg storage: u8; layout little; fields {{ erg kind: u8 at 0; }} }}"
        ));
        assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed, "{attribute}");
    }
    let error = parse_error(
        "meta copy meta repr(C) pack Cell { erg storage: u8; layout little; fields { erg kind: u8 at 0; } }",
    );
    assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed);
}

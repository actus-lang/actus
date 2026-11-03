use super::parse_source;
use actus::ast::{MetaAttribute, TopLevelDecl};

#[test]
fn parses_zero_on_drop_on_structs() {
    let program = parse_source("meta repr(C) meta zero_on_drop struct Secret { value: u64, }");
    let TopLevelDecl::Struct(structure) = &program.declarations[0] else {
        panic!("expected struct")
    };
    assert!(structure.is_zero_on_drop());
    assert!(structure.metadata.contains(&MetaAttribute::ZeroOnDrop));
}

#[test]
fn rejects_duplicate_zero_on_drop_metadata() {
    let (tokens, errors) =
        actus::lexer::scan("meta zero_on_drop meta zero_on_drop struct Secret { value: u64, }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = actus::parser::parse(tokens).expect_err("duplicate metadata must be rejected");
    assert_eq!(error.code, actus::parser::ParseErrorCode::DuplicateMetadata);
}

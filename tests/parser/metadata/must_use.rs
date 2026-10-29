use super::{parse_error, parse_source};
use actus::ast::{MetaAttribute, TopLevelDecl};
use actus::diagnostics::parse_diagnostic;
use actus::parser::{ParseErrorCode, ParseErrorKind};

#[test]
fn parses_must_use_metadata_into_the_verb_ast() {
    let program =
        parse_source("meta must_use(\"check the status\") verb status() -> Int { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.metadata.contains(&MetaAttribute::MustUse("check the status".to_owned(),)));
}

#[test]
fn must_use_requires_a_non_empty_reason() {
    let error = parse_error("meta must_use(\"\") verb status() -> Int { return 0; }");
    assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
}

#[test]
fn must_use_requires_a_return_value() {
    let error = parse_error("meta must_use(\"check it\") verb status() { return; }");
    assert_eq!(error.code, ParseErrorCode::MustUseRequiresReturn);
    assert_eq!(parse_diagnostic(&error).code(), "E0018");
    assert!(matches!(error.kind, ParseErrorKind::MustUseRequiresReturn));
}

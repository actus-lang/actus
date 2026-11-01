use super::{parse_error, parse_source};
use actus::ast::{BoundedContract, MetaAttribute, TopLevelDecl};
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

#[test]
fn parses_must_use_metadata_on_role_methods() {
    let program = parse_source(
        "role Writer { meta must_use(\"inspect the write result\") verb write(abs self: Int) -> Int; }",
    );
    let TopLevelDecl::Role(role) = &program.declarations[0] else { panic!("expected role") };
    assert!(
        role.methods[0]
            .metadata
            .contains(&MetaAttribute::MustUse("inspect the write result".to_owned()))
    );
}

#[test]
fn parses_deterministic_metadata_and_rejects_duplicates() {
    let program = parse_source("meta deterministic verb stable() -> Int { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.metadata.contains(&MetaAttribute::Deterministic));

    let error =
        parse_error("meta deterministic meta deterministic verb stable() -> Int { return 0; }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn parses_realtime_metadata_and_rejects_duplicates() {
    let program = parse_source("meta realtime verb tick() -> Int { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.metadata.contains(&MetaAttribute::Realtime));

    let error = parse_error("meta realtime meta realtime verb tick() -> Int { return 0; }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn parses_realtime_metadata_on_role_methods() {
    let program = parse_source("role Clock { meta realtime verb tick(abs self: Int) -> Int; }");
    let TopLevelDecl::Role(role) = &program.declarations[0] else { panic!("expected role") };
    assert!(role.methods[0].metadata.contains(&MetaAttribute::Realtime));
}

#[test]
fn parses_bounded_metadata_and_rejects_invalid_limits() {
    let program = parse_source(
        "meta bounded(max_iterations: 1024u32, max_call_depth: 16u32, max_stack_bytes: 4096u64) verb process() -> Int { return 0; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.metadata.contains(&MetaAttribute::Bounded(BoundedContract {
        max_iterations: Some(1024),
        max_call_depth: Some(16),
        max_calls: None,
        max_allocations: None,
        max_allocation_bytes: None,
        max_stack_bytes: Some(4096),
        max_io_bytes: None,
    })));

    let duplicate = parse_error(
        "meta bounded(max_iterations: 4u32) meta bounded(max_iterations: 8u32) verb process() -> Int { return 0; }",
    );
    assert_eq!(duplicate.code, ParseErrorCode::DuplicateMetadata);

    parse_source("meta bounded(max_allocations: 0u32) verb process() -> Int { return 0; }");
}

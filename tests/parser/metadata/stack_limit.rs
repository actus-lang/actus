use super::{parse_error, parse_source};
use actus::ast::{MetaAttribute, StackLimitContract, TopLevelDecl};
use actus::diagnostics::parse_diagnostic;
use actus::parser::ParseErrorCode;

#[test]
fn parses_stack_limit_into_the_typed_verb_metadata() {
    let program =
        parse_source("meta stack_limit(bytes: 2048u32) verb control() -> Int { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.metadata.contains(&MetaAttribute::StackLimit(StackLimitContract { bytes: 2048 })));
}

#[test]
fn parses_stack_limit_on_external_verbs_and_role_methods() {
    let program = parse_source(
        "meta stack_limit(bytes: 128u32) unsafe extern \"C\" verb native() -> Int; role Worker { meta stack_limit(bytes: 256u32) verb run(abs self: Int) -> Int; }",
    );
    let TopLevelDecl::ExternalVerb(external) = &program.declarations[0] else {
        panic!("expected external verb")
    };
    assert!(
        external.metadata.contains(&MetaAttribute::StackLimit(StackLimitContract { bytes: 128 }))
    );
    let TopLevelDecl::Role(role) = &program.declarations[1] else { panic!("expected role") };
    assert!(
        role.methods[0]
            .metadata
            .contains(&MetaAttribute::StackLimit(StackLimitContract { bytes: 256 },))
    );
}

#[test]
fn rejects_duplicate_stack_limits() {
    let error = parse_error(
        "meta stack_limit(bytes: 128u32) meta stack_limit(bytes: 256u32) verb control() -> Int { return 0; }",
    );
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
    assert_eq!(parse_diagnostic(&error).code(), "E0013");
}

#[test]
fn rejects_invalid_stack_limit_arguments() {
    for source in [
        "meta stack_limit(bytes: 0u32) verb control() -> Int { return 0; }",
        "meta stack_limit(bytes: 128u64) verb control() -> Int { return 0; }",
        "meta stack_limit(bytes: 128) verb control() -> Int { return 0; }",
        "meta stack_limit(bytes: 128u32, extra: 1u32) verb control() -> Int { return 0; }",
        "meta stack_limit(limit: 128u32) verb control() -> Int { return 0; }",
    ] {
        let error = parse_error(source);
        assert_eq!(error.code, ParseErrorCode::InvalidStackLimit);
        assert_eq!(parse_diagnostic(&error).code(), "E0023");
    }
}

#[test]
fn rejects_stack_limit_on_structs() {
    let error = parse_error("meta stack_limit(bytes: 128u32) struct State { value: u32, }");
    assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed);
}

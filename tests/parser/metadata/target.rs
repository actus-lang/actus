use super::{parse_error, parse_source};
use actus::ast::{MetaAttribute, TopLevelDecl};
use actus::diagnostics::parse_diagnostic;
use actus::parser::{ParseErrorCode, ParseErrorKind};

#[test]
fn parses_target_metadata_into_the_verb_ast() {
    let program = parse_source("meta target(\"unix\") verb platform_action() { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata, vec![MetaAttribute::Target("unix".to_owned())]);
}

#[test]
fn parses_target_and_test_metadata_together() {
    let program = parse_source(
        "meta target(\"windows\") meta test verb platform_test() -> Int { return 0; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata.len(), 2);
    assert!(verb.metadata.contains(&MetaAttribute::Target("windows".to_owned())));
    assert!(verb.metadata.contains(&MetaAttribute::Test));
}

#[test]
fn rejects_unknown_target_metadata() {
    let error = parse_error("meta target(\"plan9\") verb main() { return 0; }");
    assert_eq!(error.code, ParseErrorCode::UnsupportedTargetPlatform);
    assert_eq!(parse_diagnostic(&error).code(), "E0007");
    assert!(matches!(
        error.kind,
        ParseErrorKind::UnsupportedTargetPlatform { name } if name == "plan9"
    ));
}

#[test]
fn rejects_conflicting_target_metadata() {
    let error =
        parse_error("meta target(\"unix\") meta target(\"windows\") verb main() { return 0; }");
    assert_eq!(error.code, ParseErrorCode::ConflictingTargetPlatforms);
    assert_eq!(parse_diagnostic(&error).code(), "E0010");
    assert!(matches!(
        error.kind,
        ParseErrorKind::ConflictingTargetPlatforms { first, second }
            if first == "unix" && second == "windows"
    ));
}

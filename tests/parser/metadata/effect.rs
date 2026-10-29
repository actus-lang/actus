use super::{parse_error, parse_source};
use actus::ast::{Effect, MetaAttribute, TopLevelDecl};
use actus::diagnostics::parse_diagnostic;
use actus::parser::ParseErrorCode;

#[test]
fn parses_every_supported_effect_name() {
    for effect in Effect::ALL {
        let source =
            format!("meta effect(\"{}\") verb operation() -> Int {{ return 0; }}", effect.name());
        let program = parse_source(&source);
        let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
        assert!(verb.metadata.contains(&MetaAttribute::Effect(effect)));
    }
}

#[test]
fn parses_multiple_effects_in_one_metadata_declaration() {
    let program = parse_source(
        "meta effect(\"filesystem\", \"blocking\") verb operation() -> Int { return 0; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(
        verb.metadata,
        vec![MetaAttribute::Effect(Effect::Filesystem), MetaAttribute::Effect(Effect::Blocking),]
    );
}

#[test]
fn rejects_unknown_effect_names_with_a_stable_diagnostic() {
    let error = parse_error("meta effect(\"unknown\") verb operation() -> Int { return 0; }");
    assert_eq!(error.code, ParseErrorCode::UnsupportedEffect);
    assert_eq!(parse_diagnostic(&error).code(), "E0019");
}

#[test]
fn rejects_duplicate_effects_on_one_declaration() {
    let error = parse_error(
        "meta effect(\"filesystem\") meta effect(\"filesystem\") verb operation() -> Int { return 0; }",
    );
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn rejects_non_string_effect_arguments() {
    let error = parse_error("meta effect(filesystem) verb operation() -> Int { return 0; }");
    assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
}

#[test]
fn preserves_open_visibility_for_effectful_verbs() {
    let program = parse_source("meta effect(\"filesystem\") open verb load() -> Int { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.is_open);
    assert!(verb.metadata.contains(&MetaAttribute::Effect(Effect::Filesystem)));
}

use super::{parse_error, parse_source};
use actus::ast::{LimitlessScope, MetaAttribute, TopLevelDecl};
use actus::parser::ParseErrorCode;

#[test]
fn parses_limitless_metadata_at_verb_and_file_scope() {
    let program = parse_source(
        "meta limitless(\"file\")\nmeta limitless(\"verb\") verb large() { return 0; }",
    );
    assert_eq!(program.file_metadata, vec![LimitlessScope::File]);
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata, vec![MetaAttribute::Limitless(LimitlessScope::Verb)]);
}

#[test]
fn rejects_unsupported_limitless_scope() {
    let error = parse_error("meta limitless(\"project\") verb main() { return 0; }");
    assert_eq!(error.code, ParseErrorCode::UnsupportedLimitlessScope);
}

#[test]
fn rejects_duplicate_limitless_directives() {
    let error =
        parse_error("meta limitless(\"verb\") meta limitless(\"verb\") verb main() { return 0; }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn rejects_file_scope_limitless_after_a_declaration() {
    let error = parse_error("verb first() { return 0; } meta limitless(\"file\")");
    assert_eq!(error.code, ParseErrorCode::MetadataFileScopeNotAllowed);
}

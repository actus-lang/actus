use super::{parse_error, parse_source};
use actus::ast::{MetaAttribute, TopLevelDecl};
use actus::parser::{ParseErrorCode, ParseErrorKind};

#[test]
fn parses_test_metadata_without_name_convention() {
    let program = parse_source("meta test verb arbitrary_name() -> Int { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata, vec![MetaAttribute::Test]);
}

#[test]
fn rejects_test_metadata_on_external_verbs() {
    for source in [
        "meta test extern \"C\" verb native_test() -> Int;",
        "meta fault(\"flush_failure\") meta test unsafe extern \"C\" verb native_test() -> Int;",
    ] {
        let error = parse_error(source);
        assert_eq!(error.code, ParseErrorCode::TestMetadataRequiresVerb);
    }
}

#[test]
fn rejects_duplicate_test_metadata() {
    let error = parse_error("meta test meta test verb main() { return 0; }");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn rejects_metadata_attached_to_a_non_verb_declaration() {
    let error = parse_error("meta test struct Packet { payload: Buffer, }");
    assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed);
    assert!(matches!(error.kind, ParseErrorKind::MetadataTargetNotAllowed));
}

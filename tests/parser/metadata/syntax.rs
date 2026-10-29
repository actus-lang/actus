use super::parse_error;
use actus::diagnostics::parse_diagnostic;
use actus::parser::ParseErrorCode;

#[test]
fn rejects_malformed_metadata_arguments() {
    for source in [
        "meta target(unix) verb main() { return 0; }",
        "meta fault(flush_failure) meta test verb main() { return 0; }",
        "meta limitless(verb) verb main() { return 0; }",
    ] {
        let error = parse_error(source);
        assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
    }
}

#[test]
fn rejects_unknown_metadata_attributes() {
    let error = parse_error("meta experimental verb main() { return 0; }");
    assert_eq!(error.code, ParseErrorCode::UnknownMetadata);
    assert_eq!(parse_diagnostic(&error).code(), "E0006");
}

#[test]
fn rejects_duplicate_test_and_target_metadata() {
    for source in [
        "meta test meta test verb main() { return 0; }",
        "meta target(\"unix\") meta target(\"unix\") verb main() { return 0; }",
    ] {
        let error = parse_error(source);
        assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
    }
}

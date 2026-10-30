use super::{parse_error, parse_source};
use actus::ast::{Capability, MetaAttribute, TopLevelDecl};
use actus::diagnostics::parse_diagnostic;
use actus::parser::ParseErrorCode;

#[test]
fn parses_every_supported_capability_name() {
    for capability in Capability::ALL {
        let source = format!(
            "meta capability(\"{}\") verb operation() -> Int {{ return 0; }}",
            capability.name()
        );
        let program = parse_source(&source);
        let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
        assert!(verb.metadata.contains(&MetaAttribute::Capability(capability)));
    }
}

#[test]
fn rejects_unknown_capability_names_with_a_stable_diagnostic() {
    let error = parse_error(
        "meta capability(\"unknown.platform_primitive\") verb operation() -> Int { return 0; }",
    );
    assert_eq!(error.code, ParseErrorCode::UnsupportedCapability);
    assert_eq!(parse_diagnostic(&error).code(), "E0021");
}

#[test]
fn rejects_duplicate_capability_metadata() {
    let error = parse_error(
        "meta capability(\"posix.cloexec\") meta capability(\"posix.cloexec\") verb operation() -> Int { return 0; }",
    );
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn preserves_capability_metadata_on_open_verbs() {
    let program = parse_source(
        "meta capability(\"windows.file_identity\") open verb inspect() -> Int { return 0; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.is_open);
    assert!(verb.metadata.contains(&MetaAttribute::Capability(Capability::WindowsFileIdentity,)));
}

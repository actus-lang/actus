use super::{parse_error, parse_source};
use actus::ast::{MetaAttribute, TestFault, TopLevelDecl};
use actus::parser::{ParseErrorCode, ParseErrorKind};

#[test]
fn parses_typed_publication_fault_metadata() {
    let program = parse_source(
        "meta fault(\"flush_failure\") meta test verb publication_test() -> Int { return 0; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(verb.metadata.contains(&MetaAttribute::Fault(TestFault::FlushFailure)));
    assert!(verb.metadata.contains(&MetaAttribute::Test));
}

#[test]
fn parses_every_supported_publication_fault_name() {
    let names = [
        ("staging_open_failure", TestFault::StagingOpenFailure),
        ("write_failure", TestFault::WriteFailure),
        ("short_write", TestFault::ShortWrite),
        ("flush_failure", TestFault::FlushFailure),
        ("close_failure", TestFault::CloseFailure),
        ("cleanup_failure", TestFault::CleanupFailure),
        ("rename_failure", TestFault::RenameFailure),
        ("interrupted_before_rename", TestFault::InterruptedBeforeRename),
        ("parent_open_failure", TestFault::ParentOpenFailure),
        ("parent_sync_failure", TestFault::ParentSyncFailure),
        ("interrupted_publication", TestFault::InterruptedPublication),
    ];
    for (name, expected) in names {
        let source = format!(
            "meta fault(\"{name}\") meta test verb publication_test() -> Int {{ return 0; }}"
        );
        let program = parse_source(&source);
        let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
        assert!(verb.metadata.contains(&MetaAttribute::Fault(expected)));
    }
}

#[test]
fn rejects_multiple_publication_faults_on_one_test() {
    let error = parse_error(
        "meta fault(\"flush_failure\") meta fault(\"rename_failure\") meta test verb publication_test() -> Int { return 0; }",
    );
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn rejects_fault_metadata_without_a_test_declaration() {
    let error = parse_error(
        "meta fault(\"flush_failure\") verb publication_boundary() -> Int { return 0; }",
    );
    assert_eq!(error.code, ParseErrorCode::FaultRequiresTest);
    assert!(matches!(error.kind, ParseErrorKind::FaultRequiresTest));
}

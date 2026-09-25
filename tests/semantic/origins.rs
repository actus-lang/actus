use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{Origin, SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn accepts_direct_and_derived_single_origin_returns() {
    let direct = analyze_source("verb view(abs input: Buffer) -> abs Buffer { return input; }")
        .expect("direct abs return should preserve its source");
    assert!(
        direct
            .expression_origins
            .iter()
            .any(|record| { record.origin == Origin::AbsParameter { parameter_index: 0 } })
    );

    analyze_source(
        "struct Packet { field: Buffer, } verb view(abs self: Packet) -> abs Buffer { return self.field; }",
    )
    .expect("field derivation should preserve its source origin");

    analyze_source(
        "verb inner(abs input: Buffer) -> abs Buffer { return input; } verb outer(abs input: Buffer) -> abs Buffer { return inner(input: abs input); }",
    )
    .expect("nested view calls should preserve their root origin");

    analyze_source("verb view(abs input: Buffer) -> abs Buffer { return input.raw_slice(0, 1); }")
        .expect("raw_slice should preserve its receiver origin");
}

#[test]
fn counts_only_non_scalar_abs_parameters_as_origins() {
    let error = analyze_source("verb bad(abs number: Int) -> abs Int { return number; }")
        .expect_err("scalar abs parameters are not view origins");
    assert!(matches!(error.kind, SemanticErrorKind::InvalidAbsReturnOrigin { .. }));

    let error = analyze_source(
        "verb bad(abs left: Buffer, abs right: Buffer) -> abs Buffer { return left; }",
    )
    .expect_err("multiple abs sources must be rejected");
    assert!(matches!(error.kind, SemanticErrorKind::InvalidAbsReturnOrigin { .. }));
}

#[test]
fn rejects_local_temporary_and_unknown_view_origins() {
    let temporary = analyze_source(
        "verb bad(abs input: Buffer) -> abs Buffer { erg temporary = Buffer[4]; return temporary; }",
    )
    .expect_err("a local owner cannot be returned as an abs view");
    assert!(
        matches!(temporary.kind, SemanticErrorKind::InvalidAbsReturnOrigin { reason } if reason.contains("no abs parameter"))
    );

    let unknown =
        analyze_source("verb bad(abs input: Buffer) -> abs Buffer { return unknown_view(input); }")
            .expect_err("an unknown call cannot establish view provenance");
    assert!(
        matches!(unknown.kind, SemanticErrorKind::InvalidAbsReturnOrigin { reason } if reason.contains("unknown origin"))
    );
}

#[test]
fn rejects_multiple_origins_from_an_abs_return_call() {
    let error = analyze_source(
        "extern \"C\" verb combine(abs left: Buffer, abs right: Buffer) -> abs Buffer; verb outer(abs input: Buffer, abs other: Buffer) -> abs Buffer { return combine(left: input, right: other); }",
    )
    .expect_err("multiple view roots must not be returned");
    assert!(matches!(error.kind, SemanticErrorKind::InvalidAbsReturnOrigin { .. }));
}

#[test]
fn propagates_a_returned_view_into_the_caller_scope() {
    let model = analyze_source(
        "extern \"C\" verb sub_slice(abs input: Buffer) -> abs Buffer; verb main() -> Int { erg buffer = Buffer[8]; abs view = ref sub_slice(input: abs buffer); return 0; }",
    )
    .expect("a returned view should freeze its caller owner");
    assert_eq!(model.borrows.len(), 1);
    assert!(model.cleanup_plans.iter().any(|plan| {
        plan.actions
            .iter()
            .any(|action| matches!(action, actus::semantic::CleanupAction::EndBorrow { .. }))
    }));
    let buffer = model.bindings.iter().find(|binding| binding.name == "buffer").unwrap();
    assert_eq!(buffer.access, actus::semantic::AccessState::Mutable);
}

#[test]
fn thaws_the_source_after_a_nested_view_scope() {
    let model = analyze_source(
        "extern \"C\" verb sub_slice(abs input: Buffer) -> abs Buffer; verb main() -> Int { erg buffer = Buffer[8]; { abs view = ref sub_slice(input: abs buffer); } drop(buffer); return 0; }",
    )
    .expect("the source should thaw after the view scope");
    let buffer = model.bindings.iter().find(|binding| binding.name == "buffer").unwrap();
    assert_eq!(buffer.ownership, actus::semantic::OwnershipState::Dropped);
    assert_eq!(buffer.access, actus::semantic::AccessState::Mutable);
}

#[test]
fn rejects_owner_move_and_drop_while_a_returned_view_is_live() {
    let moved = analyze_source(
        "extern \"C\" verb sub_slice(abs input: Buffer) -> abs Buffer; verb consume(dat input: Buffer) { drop(input); } verb main() -> Int { erg buffer = Buffer[8]; abs view = ref sub_slice(input: abs buffer); consume(input: dat buffer); return 0; }",
    )
    .expect_err("a frozen source cannot be moved");
    assert!(matches!(moved.kind, SemanticErrorKind::MoveFrozen { .. }));

    let dropped = analyze_source(
        "extern \"C\" verb sub_slice(abs input: Buffer) -> abs Buffer; verb main() -> Int { erg buffer = Buffer[8]; abs view = ref sub_slice(input: abs buffer); drop(buffer); return 0; }",
    )
    .expect_err("a frozen source cannot be dropped");
    assert!(matches!(dropped.kind, SemanticErrorKind::DropFrozen { .. }));
}

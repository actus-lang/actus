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

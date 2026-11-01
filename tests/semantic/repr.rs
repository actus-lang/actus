use actus::ast::Program;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program: Program = parse(tokens).expect("repr(C) fixture should parse");
    analyze(&program)
}

#[test]
fn accepts_concrete_c_structs_and_unit_enums() {
    analyze_source(
        "meta repr(C) struct Point { x: i32, y: u32, values: Array[u16, 4], } meta repr(C) enum Axis { X, Y, }",
    )
    .expect("concrete C-compatible declarations should analyze");
}

#[test]
fn accepts_packed_structs_in_grouped_and_separate_forms() {
    analyze_source(
        "meta repr(C, packed) struct Grouped { prefix: u8, value: u32, } meta repr(C) meta repr(packed) struct Separate { prefix: u8, value: u32, }",
    )
    .expect("packed concrete structs should analyze");
}

#[test]
fn rejects_packed_enums() {
    let error = analyze_source("meta repr(packed) enum Axis { Horizontal, Vertical, }")
        .expect_err("packed enum layout must not be inferred");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("only for structs"))
    );
}

#[test]
fn rejects_ownership_fields_in_c_structs() {
    let error = analyze_source("meta repr(C) struct Resource { erg handle: Int, }")
        .expect_err("ownership-qualified fields must not cross a C ABI");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("ownership role"))
    );
}

#[test]
fn rejects_generic_c_structs_and_payload_enums() {
    let generic = analyze_source("meta repr(C) struct Box[T] { value: T, }")
        .expect_err("generic C layouts must be rejected");
    assert!(
        matches!(generic.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("generic"))
    );

    let payload = analyze_source("meta repr(C) enum Message { Data(i32), }")
        .expect_err("payload enums require a tagged-union contract");
    assert!(
        matches!(payload.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("payload"))
    );
}

#[test]
fn nested_aggregate_requires_its_own_c_contract() {
    let error =
        analyze_source("struct Inner { value: i32, } meta repr(C) struct Outer { inner: Inner, }")
            .expect_err("nested non-C aggregate must not cross a C ABI");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("repr(C) contract"))
    );
}

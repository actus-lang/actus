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
fn accepts_volatile_fixed_width_register_structs() {
    analyze_source(
        "meta repr(C) meta volatile struct UartRegisters { status: u32, data: u8, ready: Bool, }",
    )
    .expect("volatile register blocks require a checked repr(C) layout");
}

#[test]
fn rejects_volatile_structs_without_c_layout() {
    let error = analyze_source("meta volatile struct Registers { status: u32, }")
        .expect_err("volatile values must declare repr(C)");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("repr(C)"))
    );
}

#[test]
fn rejects_volatile_generic_and_non_scalar_fields() {
    let generic = analyze_source("meta repr(C) meta volatile struct Registers[T] { status: T, }")
        .expect_err("volatile register blocks cannot be generic");
    assert!(
        matches!(generic.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("generic"))
    );

    let aggregate = analyze_source(
        "meta repr(C) struct Inner { value: u32, } meta repr(C) meta volatile struct Registers { inner: Inner, }",
    )
    .expect_err("volatile fields must be fixed-width scalars");
    assert!(
        matches!(aggregate.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("fixed-width integer"))
    );
}

#[test]
fn resolves_volatile_address_only_for_volatile_register_blocks() {
    analyze_source(
        "meta repr(C) meta volatile struct Registers { status: u32, } verb inspect(abs address: VolatileAddress[Registers]) -> Int { return 0; }",
    )
    .expect("typed volatile addresses should resolve to their checked target");

    let error = analyze_source(
        "meta repr(C) struct Registers { status: u32, } verb inspect(abs address: VolatileAddress[Registers]) -> Int { return 0; }",
    )
    .expect_err("volatile addresses must not target ordinary structs");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("meta volatile"))
    );
}

#[test]
fn resolves_writable_volatile_address_only_for_volatile_register_blocks() {
    analyze_source(
        "meta repr(C) meta volatile struct Registers { status: u32, } verb inspect(ins address: VolatileWriteAddress[Registers]) -> Int { return 0; }",
    )
    .expect("typed writable volatile addresses should resolve to their checked target");

    let error = analyze_source(
        "meta repr(C) struct Registers { status: u32, } verb inspect(ins address: VolatileWriteAddress[Registers]) -> Int { return 0; }",
    )
    .expect_err("writable volatile addresses must not target ordinary structs");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("meta volatile"))
    );
}

#[test]
fn accepts_packed_structs_in_grouped_and_separate_forms() {
    analyze_source(
        "meta repr(C, packed) struct Grouped { prefix: u8, value: u32, } meta repr(C) meta repr(packed) struct Separate { prefix: u8, value: u32, }",
    )
    .expect("packed concrete structs should analyze");
}

#[test]
fn accepts_single_field_transparent_structs() {
    analyze_source("meta repr(transparent) struct UserId { value: u64, }")
        .expect("single-field transparent structs should analyze");
}

#[test]
fn accepts_explicit_alignment_and_rejects_unsafe_combinations() {
    analyze_source("meta repr(C, align(8u32)) struct Aligned { value: u8, }")
        .expect("power-of-two explicit alignment should analyze");

    let non_power = analyze_source("meta repr(C, align(3u32)) struct Invalid { value: u8, }")
        .expect_err("non-power-of-two alignment must be rejected");
    assert!(
        matches!(non_power.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("power of two"))
    );

    let packed = analyze_source("meta repr(C, packed, align(8u32)) struct Invalid { value: u8, }")
        .expect_err("packed and explicit alignment must not be combined");
    assert!(
        matches!(packed.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("packed"))
    );
}

#[test]
fn rejects_invalid_transparent_contracts() {
    let fields = analyze_source("meta repr(transparent) struct Pair { left: u32, right: u32, }")
        .expect_err("transparent structs must contain exactly one field");
    assert!(
        matches!(fields.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("exactly one field"))
    );

    let enum_error = analyze_source("meta repr(transparent) enum UserId { Value, }")
        .expect_err("transparent enums must not be inferred");
    assert!(
        matches!(enum_error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("only for structs"))
    );

    let combined = analyze_source("meta repr(C, transparent) struct UserId { value: u64, }")
        .expect_err("transparent must not be combined with repr(C)");
    assert!(
        matches!(combined.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("cannot be combined"))
    );
}

#[test]
fn accepts_transparent_wrappers_inside_checked_structs() {
    analyze_source(
        "meta repr(transparent) struct UserId { value: u64, } meta repr(C) struct Request { user: UserId, }",
    )
    .expect("transparent wrappers should provide a checked nested layout");
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
fn rejects_generic_c_structs_and_accepts_payload_enums() {
    let generic = analyze_source("meta repr(C) struct Box[T] { value: T, }")
        .expect_err("generic C layouts must be rejected");
    assert!(
        matches!(generic.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("generic"))
    );

    analyze_source("meta repr(C) enum Message { Data(i32), Empty, }")
        .expect("payload enums with fixed value fields should analyze");
}

#[test]
fn nested_aggregate_requires_its_own_c_contract() {
    let error =
        analyze_source("struct Inner { value: i32, } meta repr(C) struct Outer { inner: Inner, }")
            .expect_err("nested non-C aggregate must not cross a C ABI");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidReprC { ref reason, .. } if reason.contains("representation contract"))
    );
}

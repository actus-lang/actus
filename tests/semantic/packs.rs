use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

#[test]
fn accepts_fully_covered_u8_u16_and_u32_packs() {
    for source in [
        "pack Byte { erg storage: u8; layout little; fields { erg low: u4 at 0; abs high: u4 at 4; } }",
        "pack Word { erg storage: u16; layout big; fields { erg low: u8 at 0; abs high: u8 at 8; } }",
        "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs mode: u3 at 1; erg channel: u5 at 4; abs _reserved: u23 at 9 = 0; } }",
    ] {
        analyze_source(source).expect("fully covered pack should validate");
    }
}

#[test]
fn rejects_unsupported_pack_storage_types() {
    for source in [
        "pack Signed { erg storage: i32; layout little; fields { erg all: u32 at 0; } }",
        "pack Float { erg storage: f32; layout little; fields { erg all: u32 at 0; } }",
    ] {
        assert!(matches!(
            analyze_source(source),
            Err(SemanticErrorKind::InvalidPackStorage { .. })
        ));
    }
}

#[test]
fn rejects_pack_field_overlaps_with_bit_details() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg first: u4 at 0; abs second: u4 at 3; } }",
    )
    .expect_err("overlapping fields must be rejected");
    assert!(matches!(
        error,
        SemanticErrorKind::PackFieldOverlap { field, other, start: 3, end: 4, .. }
            if field == "second" && other == "first"
    ));
}

#[test]
fn rejects_pack_fields_beyond_storage_capacity() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg first: u4 at 0; abs second: u5 at 4; } }",
    )
    .expect_err("out-of-bounds fields must be rejected");
    assert!(matches!(
        error,
        SemanticErrorKind::PackFieldOutOfBounds { field, offset: 4, width: 5, capacity: 8, .. }
            if field == "second"
    ));
}

#[test]
fn rejects_uncovered_bits_without_reserved_field() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg first: u3 at 0; abs second: u4 at 4; } }",
    )
    .expect_err("gaps must require an explicit reserved field");
    assert!(matches!(error, SemanticErrorKind::PackUncoveredBits { start: 3, end: 4, .. }));
}

#[test]
fn accepts_pack_literals_and_erg_field_assignment() {
    analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg enabled: u1 at 0; abs _reserved: u7 at 1; } } verb main() -> Int { erg control = Control { storage: 0, }; control.enabled = 1; return control.enabled; }",
    )
    .expect("pack storage and erg fields should be usable");
}

#[test]
fn rejects_writes_to_abs_pack_fields() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { abs mode: u1 at 0; abs _reserved: u7 at 1; } } verb main() -> Int { erg control = Control { storage: 0, }; control.mode = 1; return 0; }",
    )
    .expect_err("abs pack fields must remain read-only");
    assert!(
        matches!(error, SemanticErrorKind::InvalidFieldAssignmentTarget { field } if field == "mode")
    );
}

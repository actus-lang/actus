use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn accepts_checked_integer_casts_and_unsigned_indices() {
    analyze_source(
        "verb main(abs bytes: Buffer, abs array: Array[Int, 4], abs index: u32) -> u8 { return bytes[index]; }",
    )
    .expect("u32 must be a valid index");
    analyze_source(
        "verb main(abs values: Array[Int, 4], abs index: Usize) -> Int { return values[index]; }",
    )
    .expect("Usize must be a valid index");
    analyze_source("verb main() -> u8 { erg value = 7; return value as u8; }")
        .expect("integer casts should be valid");
}

#[test]
fn rejects_incompatible_and_constant_overflow_casts() {
    let error = analyze_source("verb main(abs bytes: Buffer) { erg value = bytes as Int; }")
        .expect_err("Buffer must not cast to Int");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidPrimitiveCast { source, target } if source == "Buffer" && target == "Int")
    );

    let error = analyze_source("verb main() { erg value = 256 as u8; }")
        .expect_err("constant narrowing overflow must be rejected");
    assert!(
        matches!(error.kind, SemanticErrorKind::PrimitiveCastOutOfRange { target, literal } if target == "u8" && literal == "256")
    );
}

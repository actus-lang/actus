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

/// Analyzes `source`, expecting one error, and returns its rendered code and message together
/// with the source text its span points at. Rendering is part of the contract: an error kind
/// without a message used to panic instead of producing a diagnostic.
fn rendered_cast_error(source: &str) -> (String, String, String) {
    let error = analyze_source(source).expect_err("source must be rejected");
    let diagnostic = actus::diagnostics::semantic_diagnostic(&error);
    let span = diagnostic.span();
    (
        diagnostic.code().to_owned(),
        diagnostic.message().to_owned(),
        source[span.start..span.end].to_owned(),
    )
}

#[test]
fn constant_cast_overflow_renders_a_diagnostic_at_the_literal() {
    let cases = [
        ("verb main() { erg a: u8 = 300 as u8; }", "300", "u8", "300"),
        ("verb main() { erg a: u8 = 256 as u8; }", "256", "u8", "256"),
        ("verb main() { erg a: i8 = 200 as i8; }", "200", "i8", "200"),
        ("verb main() { erg a: u16 = (-5) as u16; }", "-5", "u16", "(-5)"),
        ("verb main() { erg a: Int = 3000000000 as Int; }", "3000000000", "Int", "3000000000"),
        ("verb main() { print(int: (300 as u8) as Int); }", "300", "u8", "300"),
        ("verb main() { erg a: i8 = -128 as i8; }", "128", "i8", "128"),
    ];
    for (source, literal, target, pointed_at) in cases {
        let (code, message, span_text) = rendered_cast_error(source);
        assert_eq!(code, "E1091", "{source}");
        assert_eq!(
            message,
            format!("literal `{literal}` is outside the range of `{target}` in a cast"),
            "{source}"
        );
        assert_eq!(span_text, pointed_at, "{source}");
    }
}

#[test]
fn constant_casts_that_fit_and_non_literal_operands_are_accepted() {
    for source in [
        "verb main() { erg a: u8 = 255 as u8; }",
        "verb main() { erg a: i8 = (-128) as i8; }",
        "verb main() { erg a: i8 = 127 as i8; }",
        "verb main() { erg a: u8 = 300 as Int as u8; }",
        "verb main() { erg a: u8 = -1 as u8; }",
    ] {
        analyze_source(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
}

#[test]
fn invalid_primitive_cast_renders_a_diagnostic() {
    let (code, message, span_text) =
        rendered_cast_error("verb main(abs bytes: Buffer) { erg v = bytes as Int; }");
    assert_eq!(code, "E1090");
    assert_eq!(message, "cannot cast `Buffer` to `Int`; casts convert between integer types");
    assert_eq!(span_text, "bytes as Int");
}

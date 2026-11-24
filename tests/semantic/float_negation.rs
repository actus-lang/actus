//! Unary `-` applies to floats and is IEEE negation; `-%` is an integer
//! wrapping operator and is rejected on floats.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

fn analyzed(statements: &str) -> Result<(), actus::semantic::SemanticError> {
    let source = format!("verb main() {{ {statements} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse")).map(|_| ())
}

#[test]
fn negation_accepts_float_literals_and_bindings() {
    for statements in [
        "erg a: f64 = -1.5f64;",
        "erg a: f32 = -(1.5f32);",
        "erg a: f64 = -1.5; erg b: f64 = -a;",
        "erg a: f64 = -0.0f64;",
    ] {
        analyzed(statements).unwrap_or_else(|error| panic!("{statements}: {error:?}"));
    }
}

#[test]
fn wrapping_negation_rejects_floats() {
    for statements in [
        "erg a: f64 = -%1.5f64;",
        "erg a: f64 = -%(1.5f64);",
        "erg a: f32 = 1.5f32; erg b: f32 = -%a;",
    ] {
        let error = analyzed(statements).expect_err(statements);
        let diagnostic = actus::diagnostics::semantic_diagnostic(&error);
        assert_eq!(diagnostic.code(), "E1025", "{statements}");
        assert!(
            diagnostic.message().contains("wrapping negation operator")
                && diagnostic.message().contains("expected `integer`"),
            "{statements}: {}",
            diagnostic.message()
        );
    }
}

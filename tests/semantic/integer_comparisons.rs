//! ADR-0120: integer operands of a comparison have one integer type, and an
//! unsuffixed literal takes the other operand's type.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

/// The code, message and source text of the one error `source` produces.
fn rendered_error(source: &str) -> (String, String, String) {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("source must be rejected");
    let diagnostic = actus::diagnostics::semantic_diagnostic(&error);
    let span = diagnostic.span();
    (
        diagnostic.code().to_owned(),
        diagnostic.message().to_owned(),
        source[span.start..span.end].to_owned(),
    )
}

fn accepts(source: &str) {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).unwrap_or_else(|error| panic!("{source}: {error:?}"));
}

fn main_with(statements: &str) -> String {
    format!("verb main() {{ {statements} }}")
}

fn mixed_message(operator: &str, left: &str, right: &str) -> String {
    format!(
        "operands of `{operator}` must have the same integer type, found `{left}` and `{right}`; convert one side explicitly with `as`"
    )
}

const COMPARISONS: [&str; 6] = ["<", "<=", ">", ">=", "==", "!="];

#[test]
fn every_comparison_rejects_mixed_widths() {
    for operator in COMPARISONS {
        let source = main_with(&format!(
            "erg a: u8 = 1u8; erg b: u32 = 2u32; erg c: Bool = a {operator} b;"
        ));
        let (code, message, span) = rendered_error(&source);
        assert_eq!(code, "E1140", "{operator}");
        assert_eq!(message, mixed_message(operator, "u8", "u32"), "{operator}");
        assert_eq!(span, "b", "{operator}");
    }
}

#[test]
fn signedness_and_named_integer_types_are_not_interchangeable() {
    let pairs = [
        ("i8", "u8"),
        ("u8", "i8"),
        ("Int", "i32"),
        ("i32", "Int"),
        ("Usize", "u64"),
        ("i8", "i64"),
    ];
    for (left, right) in pairs {
        for operator in ["<", "=="] {
            let source = main_with(&format!(
                "erg a: {left} = 1 as {left}; erg b: {right} = 2 as {right}; erg c: Bool = a {operator} b;"
            ));
            let (code, message, _) = rendered_error(&source);
            assert_eq!(code, "E1140", "{left} {operator} {right}");
            assert_eq!(message, mixed_message(operator, left, right), "{left} {operator} {right}");
        }
    }
}

#[test]
fn an_unsuffixed_literal_on_either_side_takes_the_other_operands_type() {
    for operator in COMPARISONS {
        accepts(&main_with(&format!(
            "erg a: u8 = 9u8; erg left: Bool = 5 {operator} a; erg right: Bool = a {operator} 5;"
        )));
    }
    accepts(&main_with("erg a: i8 = 9i8; erg b: Bool = -3 < a; erg c: Bool = a >= (-3);"));
    accepts(&main_with("erg a: u32 = 9u32; erg b: Bool = 0xFF == a; erg c: Bool = (4) != a;"));
}

#[test]
fn a_literal_outside_the_other_operands_range_is_rejected_at_the_literal() {
    let cases = [
        ("erg a: u8 = 1u8; erg b: Bool = a < 300;", "300"),
        ("erg a: u8 = 1u8; erg b: Bool = 300 > a;", "300"),
        ("erg a: u8 = 1u8; erg b: Bool = -1 < a;", "-1"),
        ("erg a: u8 = 1u8; erg b: Bool = a == -1;", "-1"),
        ("erg a: i8 = 1i8; erg b: Bool = 128 != a;", "128"),
    ];
    for (statements, literal) in cases {
        let (code, message, span) = rendered_error(&main_with(statements));
        assert_eq!(code, "E1068", "{statements}");
        assert!(message.contains(&format!("`{literal}`")), "{statements}: {message}");
        assert_eq!(span, literal, "{statements}");
    }
}

#[test]
fn a_cast_or_suffixed_literal_has_its_own_type() {
    let (code, message, _) =
        rendered_error(&main_with("erg a: u8 = 1u8; erg b: Bool = a < (4 as u32);"));
    assert_eq!((code.as_str(), message), ("E1140", mixed_message("<", "u8", "u32")));
    let (code, message, _) =
        rendered_error(&main_with("erg a: u8 = 1u8; erg b: Bool = 4u32 == a;"));
    assert_eq!((code.as_str(), message), ("E1140", mixed_message("==", "u32", "u8")));
    accepts(&main_with("erg a: u8 = 1u8; erg b: Bool = a < (4 as u8); erg c: Bool = a == 4u8;"));
    accepts(&main_with("erg a: u8 = 1u8; erg b: u32 = 2u32; erg c: Bool = (a as u32) < b;"));
}

#[test]
fn same_type_operands_are_accepted() {
    for operator in COMPARISONS {
        accepts(&main_with(&format!(
            "erg a: u16 = 9u16; erg b: u16 = 2u16; erg c: Bool = a {operator} b; erg d: Int = 1; erg e: Int = 2; erg f: Bool = d {operator} e;"
        )));
    }
}

#[test]
fn bool_float_and_enum_comparisons_are_unchanged() {
    accepts(&main_with("erg a: Bool = true; erg b: Bool = false; erg c: Bool = a == b;"));
    accepts(&main_with("erg a: f32 = 1.0f32; erg b: f32 = 2.0f32; erg c: Bool = a < b;"));
    accepts(
        "enum State { Ready, Busy, } verb same(erg l: State, erg r: State) -> Bool { return l == r; }",
    );
    let (code, ..) = rendered_error(&main_with(
        "erg a: f32 = 1.0f32; erg b: f64 = 2.0f64; erg c: Bool = a < b;",
    ));
    assert_eq!(code, "E1025");
    let (code, ..) =
        rendered_error(&main_with("erg a: Bool = true; erg b: u8 = 1u8; erg c: Bool = a == b;"));
    assert_eq!(code, "E1025");
    let (code, ..) = rendered_error(&main_with("erg a: f32 = 1.0f32; erg c: Bool = a < 1;"));
    assert_eq!(code, "E1025");
}

#[test]
fn a_mismatched_constant_is_reported_at_the_use_not_the_declaration() {
    let prefix = "const LIMIT: u16 = 48u16; verb main() { erg a: u32 = 5u32; ";
    let (code, _, span) = rendered_error(&format!("{prefix}erg c: Bool = a == LIMIT; }}"));
    assert_eq!((code.as_str(), span.as_str()), ("E1140", "LIMIT"));
    let (code, _, span) = rendered_error(&format!("{prefix}erg c = a + LIMIT; }}"));
    assert_eq!((code.as_str(), span.as_str()), ("E1140", "LIMIT"));
    let source = format!("{prefix}erg c: Bool = a == LIMIT; }}");
    let use_site = source.rfind("LIMIT").expect("use site");
    let (tokens, _) = scan(&source);
    let error = analyze(&parse(tokens).expect("parse")).expect_err("rejected");
    let span = actus::diagnostics::semantic_diagnostic(&error).span();
    assert_eq!(span.start, use_site);
}

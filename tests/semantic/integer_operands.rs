//! ADR-0119: both operands of an arithmetic or bitwise operator have one
//! integer type, and an unsuffixed literal takes the other operand's type.
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

const BINARY_OPERATORS: [&str; 15] =
    ["+", "-", "*", "/", "%", "+%", "-%", "*%", "+|", "-|", "*|", "&", "|", "^", "<<"];
const SAME_TYPE_OPERATORS: [&str; 14] =
    ["+", "-", "*", "/", "%", "+%", "-%", "*%", "+|", "-|", "*|", "&", "|", "^"];

#[test]
fn every_operator_family_rejects_mixed_integer_types() {
    for operator in SAME_TYPE_OPERATORS {
        let source =
            main_with(&format!("erg a: u8 = 1u8; erg b: u32 = 2u32; erg c = a {operator} b;"));
        let (code, message, span) = rendered_error(&source);
        assert_eq!(code, "E1140", "{operator}");
        assert_eq!(message, mixed_message(operator, "u8", "u32"), "{operator}");
        assert_eq!(span, "b", "{operator}");
    }
}

#[test]
fn the_reported_pairs_are_rejected_with_both_types_named() {
    let pairs = [
        ("u8", "Int"),
        ("Int", "u8"),
        ("u32", "i32"),
        ("i8", "u8"),
        ("Int", "i32"),
        ("Usize", "u64"),
        ("u16", "u8"),
        ("i64", "i32"),
    ];
    for (left, right) in pairs {
        let source = main_with(&format!(
            "erg a: {left} = 1 as {left}; erg b: {right} = 2 as {right}; erg c = a + b;"
        ));
        let (code, message, _) = rendered_error(&source);
        assert_eq!(code, "E1140", "{left} + {right}");
        assert_eq!(message, mixed_message("+", left, right), "{left} + {right}");
    }
}

#[test]
fn compound_assignments_reject_mixed_integer_types() {
    for operator in SAME_TYPE_OPERATORS {
        let source = main_with(&format!("erg a: u8 = 1u8; erg b: Int = 2; a {operator}= b;"));
        let (code, message, span) = rendered_error(&source);
        assert_eq!(code, "E1140", "{operator}=");
        assert_eq!(message, mixed_message(operator, "u8", "Int"), "{operator}=");
        assert_eq!(span, "b", "{operator}=");
    }
}

#[test]
fn compound_assignments_to_fields_and_elements_reject_mixed_integer_types() {
    let field = "struct Counter { value: u8, } verb main() { erg c = Counter { value: 1u8, }; erg big: u32 = 2u32; c.value += big; }";
    let (code, message, _) = rendered_error(field);
    assert_eq!((code.as_str(), message), ("E1140", mixed_message("+", "u8", "u32")));

    let element = "verb main() { erg values: Array[u8, 2] = Array[u8, 2](); erg big: u32 = 2u32; values[0u8] *= big; }";
    let (code, message, _) = rendered_error(element);
    assert_eq!((code.as_str(), message), ("E1140", mixed_message("*", "u8", "u32")));
}

#[test]
fn same_type_operands_are_accepted_for_every_operator() {
    for operator in BINARY_OPERATORS {
        accepts(&main_with(&format!("erg a: u8 = 9u8; erg b: u8 = 2u8; erg c = a {operator} b;")));
        accepts(&main_with(&format!("erg a: u8 = 9u8; erg b: u8 = 2u8; a {operator}= b;")));
    }
    for operator in SAME_TYPE_OPERATORS {
        accepts(&main_with(&format!("erg d: Int = 9; erg e: Int = 2; erg f = d {operator} e;")));
    }
}

#[test]
fn an_unsuffixed_literal_takes_the_other_operands_type() {
    for operator in SAME_TYPE_OPERATORS {
        accepts(&main_with(&format!(
            "erg a: u8 = 9u8; erg right: u8 = a {operator} 2; erg left: u8 = 100 {operator} a;"
        )));
        accepts(&main_with(&format!("erg a: u8 = 9u8; a {operator}= 2;")));
    }
    accepts(&main_with("erg a: i8 = 9i8; erg b: i8 = a + -3; erg c: i8 = -3 + a;"));
    accepts(&main_with("erg a: u32 = 9u32; erg b: u32 = a + 0xFF; erg c: u32 = (4) * a;"));
}

#[test]
fn a_literal_on_the_left_gives_the_expression_the_right_operands_type() {
    accepts(&main_with("erg a: u8 = 9u8; erg b: u8 = 1 + a;"));
    let (code, ..) = rendered_error(&main_with("erg a: u8 = 9u8; erg b: Int = 1 + a;"));
    assert_eq!(code, "E1027");
    let (code, message, _) =
        rendered_error(&main_with("erg a: u8 = 9u8; erg b: u32 = 2u32; erg c = (1 + a) + b;"));
    assert_eq!((code.as_str(), message), ("E1140", mixed_message("+", "u8", "u32")));
}

#[test]
fn a_literal_outside_the_other_operands_range_is_still_rejected() {
    let cases = [
        ("erg a: u8 = 1u8; erg b = a + 300;", "300"),
        ("erg a: u8 = 1u8; erg b = 300 + a;", "300"),
        ("erg a: u8 = 1u8; a += 256;", "256"),
        ("erg a: u8 = 1u8; erg b = a & 256;", "256"),
        ("erg a: i8 = 1i8; erg b = a * 128;", "128"),
        ("erg a: u8 = 1u8; erg b = a - -1;", "-1"),
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
    let (code, message, _) = rendered_error(&main_with("erg a: u8 = 1u8; erg b = a + (4 as u32);"));
    assert_eq!((code.as_str(), message), ("E1140", mixed_message("+", "u8", "u32")));
    let (code, message, _) = rendered_error(&main_with("erg a: u8 = 1u8; erg b = 4u32 + a;"));
    assert_eq!((code.as_str(), message), ("E1140", mixed_message("+", "u32", "u8")));
    accepts(&main_with("erg a: u8 = 1u8; erg b = a + (4 as u8); erg c = a + 4u8;"));
    accepts(&main_with("erg a: u8 = 1u8; erg b: u32 = 2u32; erg c = (a as u32) + b;"));
}

#[test]
fn shift_counts_keep_their_own_rule() {
    accepts(&main_with("erg a: u8 = 1u8; erg n: u32 = 2u32; erg b = a << n; erg c = a >> n;"));
    accepts(&main_with("erg a: u8 = 1u8; erg n: u32 = 2u32; a <<= n; a >>= n;"));
    accepts(&main_with("erg n: u8 = 2u8; erg b = 1 << n;"));
    let (code, ..) =
        rendered_error(&main_with("erg a: u8 = 1u8; erg n: i32 = 2i32; erg b = a << n;"));
    assert_eq!(code, "E1025");
}

#[test]
fn non_integer_operands_keep_the_operator_type_mismatch() {
    let (code, message, _) =
        rendered_error(&main_with("erg a: u8 = 1u8; erg b: Bool = true; erg c = a + b;"));
    assert_eq!(code, "E1025");
    assert!(message.contains("`Bool`"), "{message}");
}

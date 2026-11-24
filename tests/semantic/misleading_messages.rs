//! Bug #20: five diagnostics named the wrong problem. Each test pins the
//! code, the message and the exact source span.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

/// The code, message and spanned source text of the one error `source` produces.
fn semantic_error(source: &str) -> (String, String, String) {
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

fn parse_error(source: &str) -> (String, String, String) {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("source must not parse");
    let diagnostic = actus::diagnostics::parse_diagnostic(&error);
    let span = diagnostic.span();
    (
        diagnostic.code().to_owned(),
        diagnostic.message().to_owned(),
        source[span.start..span.end].to_owned(),
    )
}

fn operands_mismatch(operator: &str, expected: &str, found: &str) -> String {
    format!("type mismatch for `operands` in `{operator}`: expected `{expected}`, found `{found}`")
}

#[test]
fn assigning_to_a_constant_names_the_constant() {
    for statement in ["LIMIT = 6;", "LIMIT += 6;"] {
        let source = format!("const LIMIT: Int = 5; verb main() {{ {statement} }}");
        let (code, message, span) = semantic_error(&source);
        assert_eq!(code, "E1142", "{statement}");
        assert_eq!(
            message,
            "cannot assign to constant `LIMIT`: constants are fixed at compile time; declare an `erg` binding for a value that changes"
        );
        assert_eq!(span, "LIMIT", "{statement}");
    }
}

#[test]
fn assigning_to_an_undeclared_name_is_still_undeclared() {
    let (code, message, span) = semantic_error("verb main() { LIMIT = 6; }");
    assert_eq!((code.as_str(), message.as_str()), ("E1003", "undeclared identifier `LIMIT`"));
    assert_eq!(span, "LIMIT = 6;", "an undeclared target is reported at its statement");
}

#[test]
fn a_non_bool_if_condition_is_an_if_condition_error() {
    let (code, message, span) = semantic_error("verb main() { erg n: Int = 1; if n { n = 2; } }");
    assert_eq!(code, "E1063");
    assert_eq!(message, "`if` condition must be `Bool`, found `Int`");
    assert_eq!(span, "n");
}

#[test]
fn a_non_bool_else_if_condition_is_an_if_condition_error() {
    let source = "verb main() { erg n: Int = 1; if n == 1 { n = 2; } else if n { n = 3; } }";
    let (code, message, span) = semantic_error(source);
    assert_eq!(code, "E1063");
    assert_eq!(message, "`if` condition must be `Bool`, found `Int`");
    assert_eq!(span, "n");
    let (_, _, tail) = semantic_error(
        "verb main() { erg n: Int = 1; if n == 1 { n = 2; } else if n + 0 { n = 3; } }",
    );
    assert_eq!(tail, "n + 0");
}

#[test]
fn a_non_bool_case_guard_stays_a_case_guard_error() {
    let source = "enum Color { Red, Green, } verb inspect(erg count: Int, erg color: Color) { case abs color { Color.Red if count => 1, _ => 0, }; }";
    let (code, message, _) = semantic_error(source);
    assert_eq!(code, "E1063");
    assert_eq!(message, "case guard must return `Bool`, found `Int`");
}

#[test]
fn float_arithmetic_names_the_integer_requirement() {
    let cases = [
        ("x + 2.0f64", "addition operator", "x"),
        ("x + 2.0", "addition operator", "x"),
        ("2.0 + x", "addition operator", "2.0"),
        ("x - a", "subtraction operator", "x"),
        ("x * x", "multiplication operator", "x"),
        ("x / 2.0f64", "division operator", "x"),
        ("x % x", "remainder operator", "x"),
        ("x +% x", "wrapping addition operator", "x"),
    ];
    for (expression, operator, span) in cases {
        let source = format!(
            "verb main() {{ erg x: f64 = 1.0f64; erg a: f32 = 1.0f32; erg y = {expression}; }}"
        );
        let (code, message, found_span) = semantic_error(&source);
        assert_eq!(code, "E1025", "{expression}");
        assert_eq!(message, operands_mismatch(operator, "integer", "f64"), "{expression}");
        assert_eq!(found_span, span, "{expression}");
    }
}

#[test]
fn float_compound_assignment_names_the_integer_requirement() {
    let source = "verb main() { erg x: f64 = 1.0f64; x += 2.0f64; }";
    let (code, message, span) = semantic_error(source);
    assert_eq!(code, "E1025");
    assert_eq!(message, operands_mismatch("addition operator", "integer", "f64"));
    assert_eq!(span, "x += 2.0f64;", "a compound assignment is reported at its statement");
}

#[test]
fn operands_that_differ_still_report_both_types() {
    let source = "verb main() { erg n: i32 = 1i32; erg y = n + 2.0f64; }";
    let (code, message, span) = semantic_error(source);
    assert_eq!(code, "E1025");
    assert_eq!(message, operands_mismatch("addition operator", "i32", "f64"));
    assert_eq!(span, "2.0f64");
    let source = "verb main() { erg x: f64 = 1.0f64; erg a: f32 = 1.0f32; erg b = x < a; }";
    let (_, message, span) = semantic_error(source);
    assert_eq!(message, operands_mismatch("relational operator", "f64", "f32"));
    assert_eq!(span, "a");
}

#[test]
fn identical_operand_types_never_read_expected_equals_found() {
    let cases = [
        ("t + t", "addition operator", "integer", "Bool"),
        ("t < t", "relational operator", "integer or float", "Bool"),
        ("n && n", "logical and operator", "Bool", "i32"),
        ("n << n", "left shift operator", "unsigned integer", "i32"),
    ];
    for (expression, operator, expected, found) in cases {
        let source = format!(
            "verb main() {{ erg n: i32 = 1i32; erg t: Bool = true; erg y = {expression}; }}"
        );
        let (code, message, _) = semantic_error(&source);
        assert_eq!(code, "E1025", "{expression}");
        assert_eq!(message, operands_mismatch(operator, expected, found), "{expression}");
    }
}

#[test]
fn changing_a_frozen_binding_says_it_is_frozen() {
    let source = "verb broken(erg buffer: Buffer) { abs view = ref buffer; buffer = make(); }";
    let (code, message, span) = semantic_error(source);
    assert_eq!(code, "E1011");
    assert_eq!(
        message,
        "cannot assign to frozen binding `buffer`: it is borrowed by a live `abs` view and cannot change until the view ends"
    );
    assert_eq!(span, "buffer = make();");
}

#[test]
fn moving_a_frozen_binding_still_says_move() {
    let source = "verb consume(dat packet: Buffer) { drop(packet); } verb caller(erg buffer: Buffer) { abs view = ref buffer; consume(packet: buffer); }";
    let (code, message, _) = semantic_error(source);
    assert_eq!(code, "E1011");
    assert_eq!(message, "cannot move frozen binding `buffer`");
}

#[test]
fn while_is_reported_as_a_foreign_loop_keyword() {
    for condition in ["i < 3", "(i < 3)", "true", ""] {
        let source = format!("verb main() {{ erg i: Int = 0; while {condition} {{ i += 1; }} }}");
        let (code, message, span) = parse_error(&source);
        assert_eq!(code, "E0009", "{condition}");
        assert_eq!(
            message,
            "`while` is not an Actus keyword or loop form; write `loop { ... }` and leave it with `break`, or use `for erg index in start..end { ... }` or `repeat erg index in start .. end { ... }`",
            "{condition}"
        );
        assert_eq!(span, "while", "{condition}");
    }
}

#[test]
fn a_binding_named_while_is_not_taken_for_a_loop() {
    let (code, _, _) = parse_error("verb main() { while = ; }");
    assert_eq!(code, "E0003");
}

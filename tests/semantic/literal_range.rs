//! Bugs #84 and #85: an integer literal must fit `Int` and `Usize` like every other
//! integer type, and a constant's literal value must have the constant's type.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

fn error_code(source: &str) -> Option<(String, String)> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    analyze(&program).err().map(|error| {
        let diagnostic = actus::diagnostics::semantic_diagnostic(&error);
        (diagnostic.code().to_owned(), diagnostic.message().to_owned())
    })
}

fn fill(template: &str, type_name: &str, literal: &str) -> String {
    template.replace("{T}", type_name).replace("{L}", literal)
}

/// Contexts that take a literal of any integer type; `{T}` is the type, `{L}` the literal.
const VALUE_CONTEXTS: [&str; 17] = [
    "verb main() { erg a: {T} = {L}; }",
    "verb main() { erg a: {T} = ({L}); }",
    "verb main() { erg a: {T} = 1; a = {L}; }",
    "verb main() { erg a: {T} = 1; a += {L}; }",
    "verb f(abs x: {T}) -> Int { return 1; } verb main() { erg a = f(x: abs {L}); }",
    "verb f() -> {T} { return {L}; } verb main() { }",
    "verb main() { erg a: {T} = 1; erg b = a + {L}; }",
    "verb main() { erg a: {T} = 1; erg b = {L} + a; }",
    "verb main() { erg a: {T} = 1; erg b = a < {L}; }",
    "verb main() { erg a: {T} = 1; erg b = {L} < a; }",
    "verb main() { erg a: {T} = 1; erg b = a == {L}; }",
    "verb main() { erg a: {T} = 1; erg b = {L} == a; }",
    "verb main() { erg a: Array[{T}, 2] = Array[{T}, 2](); a[1] = {L}; }",
    "verb main() { erg a: Array[{T}, 2] = Array[{T}, 2](); a[1] += {L}; }",
    "const A: {T} = {L}; verb main() { }",
    "const B: {T} = 1; const A: {T} = B + {L}; verb main() { }",
    "verb n(abs a: {T}) -> Int { return case a { 1 => 1, _ => 0, }; } verb main() { erg b: {T} = case 1 { _ => {L}, }; }",
];

/// Contexts where only `Int` takes an unsuffixed literal (a field, a payload, an `if` branch).
const INT_VALUE_CONTEXTS: [&str; 10] = [
    "struct S { a: Int } verb main() { erg s = S { a: {L} }; }",
    "struct S { a: Int } verb main() { erg s = S { a: 1 }; s.a = {L}; }",
    "struct S { a: Int } verb main() { erg s = S { a: 1 }; s.a += {L}; }",
    "struct S { a: Int } verb main() { erg s = S { a: 1 }; erg b = s.a < {L}; }",
    "enum E { A(Int), } verb main() { erg e = E.A({L}); }",
    "verb main() { erg a: Int = if true { {L} } else { 1 }; }",
    "verb main() { erg a: Int = if true { 1 } else { {L} }; }",
    "verb main() { {L}; }",
    "verb main() { erg b = Buffer[{L}]; }",
    "verb main() { erg b = Buffer[1]; erg c = b[{L}]; }",
];

const INT_OUT_OF_RANGE: [&str; 5] =
    ["3000000000", "2147483648", "-2147483649", "0xFFFFFFFF", "-3000000000"];
const INT_IN_RANGE: [&str; 4] = ["-2147483648", "2147483647", "0", "-1"];

#[test]
fn int_literal_out_of_range_is_rejected_in_every_context() {
    let contexts = VALUE_CONTEXTS.iter().chain(&INT_VALUE_CONTEXTS);
    for template in contexts {
        for literal in INT_OUT_OF_RANGE {
            let source = fill(template, "Int", literal);
            let (code, message) =
                error_code(&source).unwrap_or_else(|| panic!("accepted: {source}"));
            assert_eq!(code, "E1068", "{source}: {message}");
            assert!(message.contains(&format!("`{literal}`")), "{source}: {message}");
            assert!(message.contains("`Int`"), "{source}: {message}");
        }
    }
}

#[test]
fn int_literal_in_range_is_accepted_in_every_context() {
    for template in VALUE_CONTEXTS.iter().chain(&INT_VALUE_CONTEXTS) {
        for literal in INT_IN_RANGE {
            let source = fill(template, "Int", literal);
            assert_eq!(error_code(&source), None, "{source}");
        }
    }
}

#[test]
fn usize_literal_out_of_range_is_rejected_in_every_context() {
    for template in VALUE_CONTEXTS {
        for literal in ["18446744073709551616", "-1"] {
            let source = fill(template, "Usize", literal);
            let (code, message) =
                error_code(&source).unwrap_or_else(|| panic!("accepted: {source}"));
            assert_eq!(code, "E1068", "{source}: {message}");
            assert!(message.contains("`Usize`"), "{source}: {message}");
        }
    }
}

#[test]
fn usize_literal_in_range_is_accepted_in_every_context() {
    for template in VALUE_CONTEXTS {
        for literal in ["0", "2147483648", "18446744073709551615"] {
            let source = fill(template, "Usize", literal);
            assert_eq!(error_code(&source), None, "{source}");
        }
    }
}

#[test]
fn int_pattern_literal_out_of_range_is_a_pattern_mismatch() {
    let template =
        "verb n(abs a: {T}) -> Int { return case a { {L} => 1, _ => 0, }; } verb main() { }";
    let rejected = [
        ("Int", "3000000000"),
        ("Int", "2147483648"),
        ("Int", "-2147483649"),
        ("Usize", "18446744073709551616"),
    ];
    for (type_name, literal) in rejected {
        let source = fill(template, type_name, literal);
        let (code, _) = error_code(&source).unwrap_or_else(|| panic!("accepted: {source}"));
        assert_eq!(code, "E1048", "{source}");
    }
    for (type_name, literal) in
        [("Int", "-2147483648"), ("Int", "2147483647"), ("Usize", "18446744073709551615")]
    {
        let source = fill(template, type_name, literal);
        assert_eq!(error_code(&source), None, "{source}");
    }
}

#[test]
fn indexed_element_assignment_checks_the_literal_against_the_element_type() {
    let array = "verb main() { erg a: Array[u8, 2] = Array[u8, 2](); a[1] = {L}; }";
    let buffer = "verb main() { erg b = Buffer[2]; b[0] = {L}; }";
    for template in [array, buffer] {
        for literal in ["300", "256", "-1"] {
            let source = fill(template, "", literal);
            let (code, _) = error_code(&source).unwrap_or_else(|| panic!("accepted: {source}"));
            assert_eq!(code, "E1068", "{source}");
        }
        for literal in ["0", "255"] {
            assert_eq!(error_code(&fill(template, "", literal)), None);
        }
    }
}

#[test]
fn constant_value_must_have_the_declared_type() {
    let mismatches = [
        ("Bool", "1"),
        ("Bool", "(1)"),
        ("Bool", "1.5"),
        ("f32", "1"),
        ("f64", "1"),
        ("f32", "-1"),
        ("f32", "true"),
        ("f32", "1.5f64"),
        ("f64", "1.5f32"),
        ("Buffer", "1"),
        ("String", "1"),
        ("Void", "1"),
        ("Usize", "true"),
        ("Int", "true"),
        ("Int", "1.5"),
        ("Int", "1u8"),
        ("u8", "1u16"),
        ("u8", "true"),
        ("Array[Int, 2]", "1"),
    ];
    for (type_name, value) in mismatches {
        let source = format!("const A: {type_name} = {value}; verb main() {{ }}");
        let (code, message) = error_code(&source).unwrap_or_else(|| panic!("accepted: {source}"));
        assert_eq!(code, "E1027", "{source}: {message}");
        assert!(message.contains("`A`"), "{source}: {message}");
    }
}

#[test]
fn constant_accepts_a_value_of_every_primitive_type() {
    let integers = [
        "Int", "Usize", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "u1",
        "i7",
    ];
    let mut sources: Vec<String> =
        integers.iter().map(|type_name| format!("const A: {type_name} = 1;")).collect();
    sources.extend(
        [
            "const A: Bool = true;",
            "const A: Bool = false;",
            "const A: f32 = 1.5;",
            "const A: f32 = 1.5f32;",
            "const A: f64 = 1.5;",
            "const A: f64 = 1.5f64;",
            "const A: u8 = 255u8;",
            "const A: i8 = -1;",
            "const A: i8 = -128;",
            "const A: Int = (7);",
            "const A: Int = -2147483648;",
            "const A: Int = 2147483647;",
            "const A: Usize = 18446744073709551615;",
            "const A: u128 = 340282366920938463463374607431768211455;",
        ]
        .map(str::to_owned),
    );
    for declaration in sources {
        let source = format!("{declaration} verb main() {{ }}");
        assert_eq!(error_code(&source), None, "{source}");
    }
}

#[test]
fn constant_literal_outside_its_type_range_is_e1068() {
    for (type_name, value) in
        [("i8", "128"), ("u8", "-1"), ("Int", "2147483648"), ("Usize", "18446744073709551616")]
    {
        let source = format!("const A: {type_name} = {value}; verb main() {{ }}");
        let (code, _) = error_code(&source).unwrap_or_else(|| panic!("accepted: {source}"));
        assert_eq!(code, "E1068", "{source}");
    }
}

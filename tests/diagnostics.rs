use actus::diagnostics::render_semantic_error;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

#[test]
fn renders_stable_semantic_error_codes_and_locations() {
    let source = "verb main() {\n    return missing;\n}\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("undeclared binding should fail");

    assert_eq!(
        render_semantic_error(source, &error),
        "error[E1003] at 2:12: undeclared identifier `missing`"
    );
}

#[test]
fn matches_semantic_diagnostic_snapshot() {
    let source = include_str!("fixtures/semantic/invalid/use_after_drop.act");
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("fixture should parse");
    let error = analyze(&program).expect_err("use after drop should fail");
    let actual = render_semantic_error(source, &error);
    let expected = include_str!("fixtures/semantic/snapshots/use_after_drop.diag.snap");

    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn renders_case_semantic_diagnostic_codes() {
    let sources = [
        (
            "enum Color { Red, Green, } verb main(erg color: Color) -> Int { return case color { Color.Red => 1, }; }",
            "E1045",
        ),
        (
            "enum Color { Red, } verb main(erg color: Color) -> Int { return case color { _ => 0, Color.Red => 1, }; }",
            "E1046",
        ),
        (
            "enum Color { Red, Green, } verb main(erg color: Color) -> Int { return case color { Color.Red => 1, Color.Red => 2, _ => 0, }; }",
            "E1047",
        ),
    ];
    for (source, code) in sources {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let program = parse(tokens).expect("source should parse");
        let error = analyze(&program).expect_err("case semantic validation should fail");
        assert!(render_semantic_error(source, &error).contains(&format!("error[{code}]")));
    }
}

#[test]
fn renders_exclusive_loan_alias_diagnostic() {
    let source = "verb merge(ins left: Buffer, abs view: Buffer) { } verb main() { erg buffer = Buffer[1]; merge(left: ins buffer, view: abs buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("ins and abs aliasing should fail");
    assert_eq!(
        render_semantic_error(source, &error),
        "error[E1065] at 1:124: exclusive loan aliases resource `buffer` more than once"
    );
}

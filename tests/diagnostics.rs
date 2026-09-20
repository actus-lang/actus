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

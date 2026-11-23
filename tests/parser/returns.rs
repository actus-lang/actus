use actus::ast::{ReturnAccess, TopLevelDecl};
use actus::lexer::scan;
use actus::parser::parse;

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

#[test]
fn parses_abs_returns_on_verbs_role_methods_and_external_verbs() {
    let program = parse_source(
        "verb view(abs source: Buffer) -> abs Slice { return source; } role Reader { verb view(abs self: Buffer) -> abs Slice; } unsafe extern \"C\" verb c_view(abs source: Buffer) -> abs Slice;",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.return_type.as_ref().unwrap().access, ReturnAccess::Abs);
    let TopLevelDecl::Role(role) = &program.declarations[1] else { panic!("expected role") };
    assert_eq!(role.methods[0].return_type.as_ref().unwrap().access, ReturnAccess::Abs);
    let TopLevelDecl::ExternalVerb(external) = &program.declarations[2] else {
        panic!("expected external verb")
    };
    assert_eq!(external.return_type.as_ref().unwrap().access, ReturnAccess::Abs);
}

#[test]
fn rejects_abs_in_non_return_type_positions() {
    let (tokens, errors) = scan("verb broken(erg input: abs Buffer) -> Int { return 0; }");
    assert!(errors.is_empty());
    assert!(parse(tokens).is_err());

    let (tokens, errors) = scan("verb broken() -> Int abs { return 0; }");
    assert!(errors.is_empty());
    assert!(parse(tokens).is_err());
}

fn parse_failure(source: &str) -> actus::parser::ParseError {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect_err("source should not parse")
}

#[test]
fn a_verb_value_without_return_names_the_fix() {
    let error = parse_failure("verb pick(erg v: Int) -> Int { v + 1 }");
    assert!(matches!(error.kind, actus::parser::ParseErrorKind::ValueWithoutReturn));
}

#[test]
fn a_trailing_if_without_return_names_the_fix() {
    let error = parse_failure("verb pick(erg v: Int) -> Int { if v > 10 { 3 } else { 1 } }");
    assert!(matches!(error.kind, actus::parser::ParseErrorKind::ValueWithoutReturn));
}

#[test]
fn a_missing_semicolon_in_a_verb_without_a_value_keeps_the_old_error() {
    for source in ["verb main() { run() }", "verb main() -> Void { run() }"] {
        let error = parse_failure(source);
        assert!(
            matches!(error.kind, actus::parser::ParseErrorKind::UnexpectedToken { .. }),
            "unexpected error for `{source}`: {:?}",
            error.kind
        );
    }
}

#[test]
fn an_if_value_keeps_its_branch_values() {
    parse_source("verb pick(erg v: Int) -> Int { erg x = if v > 10 { 3 } else { 1 }; return x; }");
}

#[test]
fn a_missing_semicolon_before_a_statement_keeps_the_old_error() {
    let error = parse_failure("verb main() { erg a = 1 erg b = 2; }");
    assert!(matches!(error.kind, actus::parser::ParseErrorKind::UnexpectedToken { .. }));

    let error = parse_failure("verb main() { run() erg b = 2; }");
    assert!(matches!(error.kind, actus::parser::ParseErrorKind::UnexpectedToken { .. }));
}

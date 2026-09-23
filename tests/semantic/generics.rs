use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program)
}

#[test]
fn resolves_generic_parameters_and_applied_types_in_declarations() {
    analyze_source("struct Box[T] { item: T, } verb consume[T](erg item: Box[T]) { }")
        .expect("generic parameters and applications should resolve");
}

#[test]
fn rejects_too_few_and_too_many_type_arguments() {
    for source in [
        "struct Box[T] { item: T, } verb main(erg item: Box) { }",
        "struct Box[T] { item: T, } verb main(erg item: Box[Int, Bool]) { }",
    ] {
        let error = analyze_source(source).expect_err("invalid generic arity must fail");
        assert!(
            matches!(error.kind, SemanticErrorKind::GenericArityMismatch { name, .. } if name == "Box")
        );
    }
}

#[test]
fn rejects_undeclared_generic_parameters_inside_a_declaration() {
    let error = analyze_source("verb wrap[T](erg item: U) { }")
        .expect_err("undeclared generic parameter must fail");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::UnknownTypeParameter { name } if name == "U"
    ));
}

#[test]
fn generic_parameter_scope_does_not_escape_its_verb() {
    let error = analyze_source("verb first[T](erg item: T) { } verb second(erg item: T) { }")
        .expect_err("generic parameters must not escape their declaration");
    assert!(matches!(error.kind, SemanticErrorKind::UnknownType { name } if name == "T"));
}

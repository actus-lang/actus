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
    analyze_source(
        "struct Box[T] { item: T, } enum Option[T] { Some(T), None, } verb consume[T](erg item: Box[T]) -> Box[T] { }",
    )
        .expect("generic parameters and applications should resolve");
}

#[test]
fn resolves_generic_enum_payloads_and_verb_return_types() {
    analyze_source(
        "struct Box[T] { item: T, } enum Result[T, E] { Ok(T), Err(E), } verb wrap[T, E](erg item: T) -> Result[Box[T], E] { }",
    )
    .expect("generic payloads and return applications should resolve");
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

#[test]
fn validates_role_bound_shape_before_role_resolution_exists() {
    analyze_source("verb write[T: Writer](erg item: T) { }")
        .expect("a simple role bound should be retained for Phase 14 resolution");
}

#[test]
fn rejects_applied_role_bounds() {
    let error = analyze_source("verb write[T: Writer[Int]](erg item: T) { }")
        .expect_err("role bounds must be simple role names");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::GenericArityMismatch { name, expected: 0, found: 1 } if name == "Writer"
    ));
}

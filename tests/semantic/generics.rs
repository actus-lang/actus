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
fn rejects_unknown_role_bounds() {
    let error = analyze_source("verb write[T: Writer](erg item: T) { }")
        .expect_err("unknown role bounds must fail");
    assert!(matches!(error.kind, SemanticErrorKind::UnknownRole { name } if name == "Writer"));
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

#[test]
fn accepts_multiple_symbolic_bounds() {
    analyze_source("role Writer { verb write(abs self: Int); } role Serializable { verb encode(abs self: Int); } verb write[T: Writer + Serializable](erg item: T) { }")
        .expect("declared role bounds should resolve");
}

#[test]
fn accepts_generic_role_bound_for_a_performed_type() {
    analyze_source("struct File { value: Int, } role Writer { verb write(abs self: File); } perform Writer for File { verb write(abs self: File) { } } struct Box[T: Writer] { item: T, } verb main(erg item: Box[File]) { }")
        .expect("a performed type should satisfy a role bound");
}

#[test]
fn rejects_generic_role_bound_without_a_performance() {
    let error = analyze_source("struct File { value: Int, } role Writer { verb write(abs self: File); } struct Box[T: Writer] { item: T, } verb main(erg item: Box[File]) { }")
        .expect_err("a type without a performance must fail its role bound");
    assert!(
        matches!(error.kind, SemanticErrorKind::GenericConstraintMismatch { parameter, constraint, argument } if parameter == "T" && constraint == "Writer" && argument == "File")
    );
}

#[test]
fn rejects_known_constraint_mismatch() {
    let error =
        analyze_source("struct Box[T: Numeric] { item: T, } verb main(erg item: Box[String]) { }")
            .expect_err("String must not satisfy Numeric");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::GenericConstraintMismatch { parameter, constraint, argument }
            if parameter == "T" && constraint == "Numeric" && argument == "String"
    ));
}

#[test]
fn discovers_sorted_concrete_generic_instances_with_canonical_keys() {
    let model = analyze_source(
        "struct Box[T] { item: T, } struct Pair[T] { item: T, } verb main(erg left: Pair[Box[Int]], erg right: Box[Int]) { }",
    )
    .expect("concrete generic applications should be accepted");
    let keys = model
        .generic_instances
        .iter()
        .map(|instance| instance.canonical_key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["Box[Int]", "Pair[Box[Int]]"]);
}

#[test]
fn rejects_direct_recursive_generic_layouts() {
    let error = analyze_source("struct Node[T] { next: Node[T], }")
        .expect_err("recursive generic layouts require indirection");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::RecursiveType { name } if name == "Node"
    ));
}

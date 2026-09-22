use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn validates_mutable_struct_field_assignment() {
    analyze_source(
        "struct Point { x: Int, } verb main() { erg point = Point { x: 1, }; point.x = 2; }",
    )
    .expect("erg struct fields should be assignable");

    let error = analyze_source(
        "struct Point { x: Int, } verb broken() { erg source = Point { x: 1, }; abs point = ref source; point.x = 2; }",
    )
    .expect_err("abs struct bindings must not be assigned");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::InvalidFieldAssignmentTarget { field } if field == "x"
    ));
}

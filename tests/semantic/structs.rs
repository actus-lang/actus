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

#[test]
fn rejects_whole_struct_use_after_dat_transfer() {
    let error = analyze_source(
        "struct Point { x: Int, } verb consume(dat point: Point) { } verb broken() { erg point = Point { x: 1, }; consume(point: point); inspect(point.x); }",
    )
    .expect_err("moved structs must not be reused");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::UseAfterMove { name } if name == "point"
    ));
}

#[test]
fn rejects_partial_struct_use_after_field_move() {
    let error = analyze_source(
        "struct Holder { erg payload: Buffer, value: Int, } verb consume(dat payload: Buffer) { drop(payload); } verb broken() { erg holder = Holder { payload: allocate(4), value: 1, }; consume(payload: holder.payload); inspect(holder.value); }",
    )
    .expect_err("partially moved structs must not be reused");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::UseAfterMove { name } if name == "holder"
    ));
}

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
fn registers_structs_and_validates_owned_literals() {
    let model = analyze_source(
        "struct Point { x: Int, y: Int, } verb main() { erg point = Point { x: 1, y: 2, }; inspect(point.x); }",
    )
    .expect("valid struct declaration and literal should pass");

    assert_eq!(model.bindings.len(), 1);
}

#[test]
fn rejects_duplicate_struct_names_and_fields() {
    let duplicate_struct = analyze_source("struct Point { x: Int, } struct Point { y: Int, }")
        .expect_err("duplicate struct names must fail");
    assert!(matches!(
        duplicate_struct.kind,
        SemanticErrorKind::DuplicateStructName { name } if name == "Point"
    ));

    let duplicate_field = analyze_source("struct Point { x: Int, x: Int, }")
        .expect_err("duplicate struct fields must fail");
    assert!(matches!(
        duplicate_field.kind,
        SemanticErrorKind::DuplicateStructField { struct_name, field }
            if struct_name == "Point" && field == "x"
    ));
}

#[test]
fn validates_struct_field_types_and_known_field_types() {
    let unknown_type =
        analyze_source("struct Point { x: Missing, }").expect_err("unknown field types must fail");
    assert!(matches!(
        unknown_type.kind,
        SemanticErrorKind::UnknownType { name } if name == "Missing"
    ));

    let mismatch = analyze_source(
        "struct Point { x: Int, } verb main() { erg point = Point { x: \"wrong\", }; }",
    )
    .expect_err("field type mismatches must fail");
    assert!(matches!(
        mismatch.kind,
        SemanticErrorKind::StructFieldTypeMismatch { struct_name, field, expected, found }
            if struct_name == "Point" && field == "x" && expected == "Int" && found == "String"
    ));
}

#[test]
fn rejects_missing_unknown_and_invalid_field_access() {
    let missing = analyze_source(
        "struct Point { x: Int, y: Int, } verb main() { erg point = Point { x: 1, }; }",
    )
    .expect_err("missing fields must fail");
    assert!(matches!(
        missing.kind,
        SemanticErrorKind::MissingStructField { struct_name, field }
            if struct_name == "Point" && field == "y"
    ));

    let unknown = analyze_source(
        "struct Point { x: Int, } verb main() { erg point = Point { x: 1, z: 2, }; }",
    )
    .expect_err("unknown initialized fields must fail");
    assert!(matches!(
        unknown.kind,
        SemanticErrorKind::UnknownStructField { struct_name, field }
            if struct_name == "Point" && field == "z"
    ));

    let access = analyze_source(
        "struct Point { x: Int, } verb main() { erg point = Point { x: 1, }; inspect(point.y); }",
    )
    .expect_err("unknown accessed fields must fail");
    assert!(matches!(
        access.kind,
        SemanticErrorKind::UnknownStructField { struct_name, field }
            if struct_name == "Point" && field == "y"
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

#[test]
fn freezes_struct_owner_for_field_borrow_and_restores_it() {
    let model = analyze_source(
        "struct Holder { payload: Buffer, value: Int, } verb main() { erg holder = Holder { payload: allocate(4), value: 1, }; { abs view = ref holder.payload; inspect(view); } holder.value = 2; }",
    )
    .expect("field borrow should end with its lexical scope");
    assert_eq!(model.borrows[0].field.as_deref(), Some("payload"));

    let error = analyze_source(
        "struct Holder { payload: Buffer, value: Int, } verb broken() { erg holder = Holder { payload: allocate(4), value: 1, }; { abs view = ref holder.payload; holder.value = 2; } }",
    )
    .expect_err("a frozen struct must reject field mutation");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::FieldBorrowConflict { owner, field, .. }
            if owner == "holder" && field == "value"
    ));
}

#[test]
fn rejects_dat_move_of_a_frozen_struct_field() {
    let error = analyze_source(
        "struct Holder { erg payload: Buffer, value: Int, } verb consume(dat payload: Buffer) { drop(payload); } verb broken() { erg holder = Holder { payload: allocate(4), value: 1, }; { abs view = ref holder.value; consume(payload: holder.payload); } }",
    )
    .expect_err("a frozen field owner must reject dat transfer");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::FieldBorrowConflict { owner, field, .. }
            if owner == "holder" && field == "payload"
    ));
}

#[test]
fn validates_method_receiver_roles_and_types() {
    analyze_source(
        "struct Point { x: Int, } verb read(abs self: Point) -> Int { return self.x; } verb main() -> Int { erg point = Point { x: 7, }; return point.read(); }",
    )
    .expect("an abs struct receiver should be valid");

    let error = analyze_source(
        "struct Point { x: Int, } verb read(erg self: Point) -> Int { return self.x; } verb main() -> Int { abs point = ref Point { x: 7, }; return point.read(); }",
    )
    .expect_err("an erg receiver must not be called through an abs binding");
    assert!(matches!(error.kind, SemanticErrorKind::InvalidArgumentRole { .. }));
}

#[test]
fn rejects_invalid_method_receivers() {
    let error = analyze_source(
        "struct Point { x: Int, } verb read(dat self: Point) -> Int { return 1; } verb main() -> Int { return 1; }",
    )
    .expect_err("dat is not a valid method receiver role");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::InvalidReceiver { method } if method == "read"
    ));
}

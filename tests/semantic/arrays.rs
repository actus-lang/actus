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
fn accepts_array_reads_and_in_place_writes() {
    analyze_source(
        "verb read(abs values: Array[Int, 4], abs index: Int) -> Int { return values[index]; }",
    )
    .expect("dynamic array reads should be semantically valid");
    analyze_source("verb write(erg values: Array[Int, 4]) { values[3] = 7; }")
        .expect("in-place array writes should be semantically valid");
}

#[test]
fn accepts_nested_array_struct_place_assignment() {
    analyze_source(
        "struct Point { x: Int, y: Int, } verb write(erg points: Array[Point, 2]) { points[1].x = 7; points[1].x += 1; }",
    )
    .expect("nested array struct places should remain writable");
}

#[test]
fn rejects_nested_mutation_through_an_abs_array_place() {
    let error = analyze_source(
        "struct Point { x: Int, } verb write(abs points: Array[Point, 2]) { points[1].x = 7; }",
    )
    .expect_err("nested places must preserve the array owner's read-only role");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::InvalidFieldAssignmentTarget { field } if field == "x"
    ));
}

#[test]
fn accepts_dynamic_buffer_reads_as_u8() {
    analyze_source("verb read(abs bytes: Buffer, abs index: Int) -> u8 { return bytes[index]; }")
        .expect("buffer indexing should produce a u8 view");
}

#[test]
fn rejects_non_integer_buffer_indices() {
    let error =
        analyze_source("verb main(abs bytes: Buffer, abs flag: Bool) { print(bytes[flag]); }")
            .expect_err("boolean buffer indices must be rejected");
    assert!(matches!(error.kind, SemanticErrorKind::InvalidIndexType { found } if found == "Bool"));
}

#[test]
fn rejects_invalid_array_capacity() {
    let error = analyze_source("verb main(abs values: Array[Int, 0]) { }")
        .expect_err("zero-sized arrays must be rejected");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidArrayCapacity { capacity } if capacity == "0")
    );
}

#[test]
fn rejects_non_const_generic_array_capacity() {
    let error = analyze_source(
        "struct Table[N: Reader] { cells: Array[Int, N], } role Reader { verb read(); } verb main() { }",
    )
    .expect_err("role type parameters must not be used as array capacities");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::InvalidArrayCapacity { capacity } if capacity == "N"
    ));
}

#[test]
fn rejects_constant_array_index_out_of_bounds() {
    let error = analyze_source("verb main(abs values: Array[Int, 4]) { print(values[4]); }")
        .expect_err("a constant index at capacity must be rejected");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::IndexOutOfBounds { index, capacity }
            if index == "4" && capacity == "4"
    ));
}

#[test]
fn rejects_non_integer_and_non_array_indices() {
    let error = analyze_source(
        "verb main(abs values: Array[Int, 4], abs flag: Bool) { print(values[flag]); }",
    )
    .expect_err("boolean indices must be rejected");
    assert!(matches!(error.kind, SemanticErrorKind::InvalidIndexType { found } if found == "Bool"));

    let error = analyze_source("verb main() { erg number = 1; print(number[0]); }")
        .expect_err("scalar values must not be indexable");
    assert!(
        matches!(error.kind, SemanticErrorKind::NonIndexableTarget { found } if found == "Int")
    );
}

#[test]
fn rejects_indexed_element_type_mismatch() {
    let error = analyze_source("verb main(erg values: Array[Int, 4]) { values[0] = \"text\"; }")
        .expect_err("indexed writes must match the element type");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::IndexedElementTypeMismatch { expected, found }
            if expected == "Int" && found == "String"
    ));
}

#[test]
fn rejects_returning_an_arena_derived_indexed_element() {
    let error = analyze_source(
        "struct Point { x: Int, } verb leak() -> Point { erg arena: Arena[128] = Arena[128](); erg points: Array[Point, 2] = arena.place(value: Array[Point, 2]()); return points[0]; }",
    )
    .expect_err("arena-derived indexed values must not escape their arena");
    assert!(matches!(error.kind, SemanticErrorKind::ArenaReferenceEscape { .. }));
}

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
fn rejects_invalid_array_capacity() {
    let error = analyze_source("verb main(abs values: Array[Int, 0]) { }")
        .expect_err("zero-sized arrays must be rejected");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidArrayCapacity { capacity } if capacity == "0")
    );
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

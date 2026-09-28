use actus::diagnostics::semantic_diagnostic;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("fixture should parse");
    analyze(&program)
}

#[test]
fn return_paths_have_positive_and_negative_evidence() {
    analyze_source("verb main() -> Int { return 42; }").expect("complete return path should pass");
    let error = analyze_source("verb main() -> Int { erg answer = 42; }")
        .expect_err("missing result return must fail");
    assert!(matches!(error.kind, SemanticErrorKind::MissingReturnValue));
    assert_eq!(semantic_diagnostic(&error).code(), "E1028");
}

#[test]
fn generic_bounds_and_instantiations_have_positive_and_negative_evidence() {
    analyze_source("role Reader { verb read(abs self: Int); } struct Box[T: Reader] { item: T, }")
        .expect("declared generic bounds should pass");
    let error =
        analyze_source("struct Box[T] { item: T, } verb main(erg item: Box[Int, Bool]) { }")
            .expect_err("unsupported generic arity must fail");
    assert!(matches!(error.kind, SemanticErrorKind::GenericArityMismatch { .. }));
    assert_eq!(semantic_diagnostic(&error).code(), "E1053");
}

#[test]
fn primitive_width_and_range_validation_have_positive_and_negative_evidence() {
    analyze_source("verb main(erg bit: u1, erg signed: i8) { }")
        .expect("supported primitive widths should pass");
    let error = analyze_source("verb main() { erg bit: u1 = 2; }")
        .expect_err("out-of-range primitive literals must fail");
    assert!(matches!(error.kind, SemanticErrorKind::NumericLiteralOutOfRange { .. }));
    assert_eq!(semantic_diagnostic(&error).code(), "E1068");
}

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
fn accepts_named_allocate_length() {
    analyze_source("verb main() { erg buffer: Buffer = allocate(length: 4); drop(buffer); }")
        .expect("named intrinsic arguments should be accepted");
}

#[test]
fn rejects_intrinsic_wrong_argument_count() {
    let error =
        analyze_source("verb main() { allocate(); }").expect_err("allocate requires one argument");
    assert!(
        matches!(error.kind, SemanticErrorKind::WrongArgumentCount { callee } if callee == "allocate")
    );

    let error =
        analyze_source("verb main() { append(1); }").expect_err("append requires two arguments");
    assert!(
        matches!(error.kind, SemanticErrorKind::WrongArgumentCount { callee } if callee == "append")
    );
}

#[test]
fn rejects_non_scalar_intrinsic_arguments() {
    let error = analyze_source("verb main() { allocate(length: \"large\"); }")
        .expect_err("allocate length must be scalar");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter } if callee == "allocate" && parameter == "length")
    );
}

#[test]
fn rejects_borrow_as_append_handle() {
    let error = analyze_source(
        "verb main() { erg buffer: Buffer = allocate(4); { abs view = ref buffer; append(view, 1); } }",
    )
    .expect_err("append must require an exclusive owner");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidArgumentRole { callee, parameter } if callee == "append" && parameter == "handle")
    );
}

#[test]
fn rejects_declarations_that_shadow_intrinsics() {
    let error =
        analyze_source("verb allocate() { }").expect_err("intrinsic names must be reserved");
    assert!(
        matches!(error.kind, SemanticErrorKind::ReservedIntrinsicName { name } if name == "allocate")
    );
}

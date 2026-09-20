use actus::ast::Role;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{BindingState, SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program)
}

#[test]
fn tracks_parameters_and_nested_borrow_records() {
    let model = analyze_source(
        "verb inspect_buffer(erg buffer: Buffer) { { abs view = ref buffer; { abs nested = ref buffer; inspect(nested); } inspect(view); } }",
    )
    .expect("source should pass semantic analysis");

    assert_eq!(model.bindings[0].role, Role::Erg);
    assert_eq!(model.borrows.len(), 2);
    assert_eq!(model.borrows[0].owner, "buffer");
    assert_eq!(model.bindings[0].state, BindingState::Active);
}

#[test]
fn rejects_undeclared_identifiers() {
    let error = analyze_source("verb broken() { inspect(missing); }")
        .expect_err("missing binding must fail");
    assert!(
        matches!(error.kind, SemanticErrorKind::UndeclaredIdentifier { name } if name == "missing")
    );
}

#[test]
fn rejects_duplicate_bindings_and_shadowing() {
    let duplicate = analyze_source("verb broken() { erg value = 1; erg value = 2; }")
        .expect_err("duplicate binding must fail");
    assert!(
        matches!(duplicate.kind, SemanticErrorKind::DuplicateBinding { name } if name == "value")
    );

    let shadow = analyze_source("verb broken(erg value: Buffer) { { erg value = 1; } }")
        .expect_err("shadowing must fail");
    assert!(matches!(shadow.kind, SemanticErrorKind::ShadowedBinding { name } if name == "value"));
}

#[test]
fn rejects_borrowing_non_owner_bindings() {
    let error = analyze_source("verb broken(abs view: Buffer) { abs nested = ref view; }")
        .expect_err("borrowing an abs binding must fail");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidBorrowTarget { name } if name == "view")
    );
}

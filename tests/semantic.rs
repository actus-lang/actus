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

#[test]
fn moves_dat_arguments_and_rejects_use_after_move() {
    let error = analyze_source(
        "verb consume(dat packet: Buffer) { drop(packet); } verb caller() { erg buffer = make(); consume(packet: buffer); inspect(buffer); }",
    )
    .expect_err("a moved binding must not be reusable");
    assert!(matches!(error.kind, SemanticErrorKind::UseAfterMove { name } if name == "buffer"));
}

#[test]
fn rejects_move_of_a_frozen_owner() {
    let error = analyze_source(
        "verb consume(dat packet: Buffer) { drop(packet); } verb caller() { erg buffer = make(); { abs view = ref buffer; consume(packet: buffer); } }",
    )
    .expect_err("a frozen binding must not be moved");
    assert!(matches!(error.kind, SemanticErrorKind::MoveFrozen { name } if name == "buffer"));
}

#[test]
fn tracks_explicit_drop_and_rejects_double_drop_and_borrow_drop() {
    let double_drop =
        analyze_source("verb broken() { erg value = make(); drop(value); drop(value); }")
            .expect_err("double drop must fail");
    assert!(matches!(double_drop.kind, SemanticErrorKind::DoubleDrop { name } if name == "value"));

    let borrow_drop = analyze_source(
        "verb broken() { erg value = make(); { abs view = ref value; drop(view); } }",
    )
    .expect_err("dropping a borrow must fail");
    assert!(matches!(borrow_drop.kind, SemanticErrorKind::DropBorrow { name } if name == "view"));
}

#[test]
fn validates_named_and_positional_call_arguments() {
    let unknown = analyze_source(
        "verb consume(dat packet: Buffer) { drop(packet); } verb caller() { erg value = make(); consume(other: value); }",
    )
    .expect_err("unknown parameter names must fail");
    assert!(
        matches!(unknown.kind, SemanticErrorKind::UnknownParameter { name, .. } if name == "other")
    );

    let mixed = analyze_source(
        "verb consume(dat packet: Buffer, erg target: Buffer) { drop(packet); } verb caller() { erg value = make(); consume(value, target: value); }",
    )
    .expect_err("mixed argument modes must fail");
    assert!(
        matches!(mixed.kind, SemanticErrorKind::MixedArgumentModes { callee } if callee == "consume")
    );
}

#[test]
fn validates_call_roles_and_rejects_ambiguous_positional_calls() {
    let invalid_abs = analyze_source(
        "verb inspect_view(abs view: Buffer) { inspect(view); } verb caller() { erg buffer = make(); inspect_view(buffer); }",
    )
    .expect_err("an owner must not be passed as an abs binding without ref");
    assert!(
        matches!(invalid_abs.kind, SemanticErrorKind::InvalidArgumentRole { parameter, .. } if parameter == "view")
    );

    let ambiguous = analyze_source(
        "verb copy(erg destination: Buffer, erg source: Buffer) { } verb caller() { erg first = make(); erg second = make(); copy(first, second); }",
    )
    .expect_err("same-role same-type positional calls must be labeled");
    assert!(
        matches!(ambiguous.kind, SemanticErrorKind::AmbiguousPositionalCall { callee } if callee == "copy")
    );
}

use actus::ast::Role;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{AccessState, OwnershipState, SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn accepts_explicit_ins_call_and_restores_the_owner() {
    let model = analyze_source(
        "verb mutate(ins buffer: Buffer) { } verb main() { erg buffer = Buffer[1]; mutate(buffer: ins buffer); inspect(buffer); }",
    )
    .expect("exclusive call should pass");
    assert_eq!(model.exclusive_loans.len(), 1);
    assert_eq!(model.exclusive_loans[0].id, 0);
    assert_eq!(model.exclusive_loans[0].owner, "buffer");
    assert_eq!(model.bindings[1].role, Role::Erg);
    assert_eq!(model.bindings[1].ownership, OwnershipState::Active);
    assert_eq!(model.bindings[1].access, AccessState::Mutable);
}

#[test]
fn allows_nested_ins_forwarding() {
    analyze_source(
        "verb inner(ins buffer: Buffer) { } verb outer(ins buffer: Buffer) { inner(buffer: ins buffer); }",
    )
    .expect("an instrumental parameter may be forwarded sequentially");
}

#[test]
fn requires_explicit_ins_at_the_call_site() {
    let error = analyze_source(
        "verb mutate(ins buffer: Buffer) { } verb main() { erg buffer = Buffer[1]; mutate(buffer: buffer); }",
    )
    .expect_err("ins must be visible at the call site");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidArgumentRole { parameter, .. } if parameter == "buffer")
    );
}

#[test]
fn rejects_ins_from_an_abs_binding() {
    let error = analyze_source(
        "verb mutate(ins buffer: Buffer) { } verb main(abs view: Buffer) { mutate(buffer: ins view); }",
    )
    .expect_err("an abs binding cannot become an exclusive loan");
    assert!(matches!(error.kind, SemanticErrorKind::InvalidArgumentRole { .. }));
}

#[test]
fn rejects_duplicate_ins_and_ins_abs_aliases() {
    let duplicate = analyze_source(
        "verb merge(ins left: Buffer, ins right: Buffer) { } verb main() { erg buffer = Buffer[1]; merge(left: ins buffer, right: ins buffer); }",
    )
    .expect_err("one root cannot receive two exclusive loans");
    assert!(
        matches!(duplicate.kind, SemanticErrorKind::ExclusiveLoanAlias { name } if name == "buffer")
    );

    let mixed = analyze_source(
        "verb merge(ins target: Buffer, abs view: Buffer) { } verb main() { erg buffer = Buffer[1]; merge(target: ins buffer, view: abs buffer); }",
    )
    .expect_err("exclusive and shared access cannot alias");
    assert!(
        matches!(mixed.kind, SemanticErrorKind::ExclusiveLoanAlias { name } if name == "buffer")
    );
}

#[test]
fn assigns_deterministic_loan_ids_and_resumes_each_call_once() {
    let model = analyze_source(
        "verb mutate(ins buffer: Buffer) { } verb main() { erg first = Buffer[1]; erg second = Buffer[1]; mutate(buffer: ins first); mutate(buffer: ins second); inspect(first); inspect(second); }",
    )
    .expect("sequential exclusive calls should pass");
    assert_eq!(model.exclusive_loans.iter().map(|loan| loan.id).collect::<Vec<_>>(), vec![0, 1]);
    assert!(model.bindings.iter().skip(1).all(|binding| {
        binding.ownership == OwnershipState::Active && binding.access == AccessState::Mutable
    }));
}

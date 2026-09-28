use actus::ast::Role;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{AccessState, OwnershipState, SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let source = format!("unsafe extern \"C\" verb inspect(abs view: Buffer); {source}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn accepts_explicit_ins_call_and_restores_the_owner() {
    let model = analyze_source(
        "verb mutate(ins buffer: Buffer) { } verb main() { erg buffer = Buffer[1]; mutate(buffer: ins buffer); inspect(view: abs buffer); }",
    )
    .expect("exclusive call should pass");
    assert_eq!(model.exclusive_loans.len(), 1);
    assert_eq!(model.exclusive_loans[0].id, 0);
    assert_eq!(model.exclusive_loans[0].owner, "buffer");
    assert_eq!(model.bindings[1].role, Role::Erg);
    assert_eq!(model.bindings[1].ownership, OwnershipState::Active);
    assert_eq!(model.bindings[1].access, AccessState::Mutable);
    assert!(model.cleanup_plans[0].actions.iter().all(|action| {
        !matches!(action, actus::semantic::CleanupAction::DropBinding { binding_index: 0 })
    }));
}

#[test]
fn allows_nested_ins_forwarding() {
    analyze_source(
        "verb inner(ins buffer: Buffer) { } verb outer(ins buffer: Buffer) { inner(buffer: ins buffer); }",
    )
    .expect("an instrumental parameter may be forwarded sequentially");
}

#[test]
fn forwards_ins_through_multiple_named_helpers() {
    let model = analyze_source(
        "verb inner(ins buffer: Buffer) -> Int { return 0; } verb middle(ins buffer: Buffer) -> Int { return inner(buffer: ins buffer); } verb outer(ins buffer: Buffer) -> Int { return middle(buffer: ins buffer); }",
    )
    .expect("nested helpers should forward the exclusive loan");
    assert_eq!(model.exclusive_loans.iter().map(|loan| loan.id).collect::<Vec<_>>(), vec![0, 1]);
}

#[test]
fn joins_case_branches_after_sequential_exclusive_loans() {
    analyze_source(
        "enum Color { Red, Green, } verb mutate(ins buffer: Buffer) { } verb main(erg color: Color) { erg buffer = Buffer[1]; case abs color { Color.Red => { mutate(buffer: ins buffer); }, Color.Green => { mutate(buffer: ins buffer); }, }; inspect(view: abs buffer); }",
    )
    .expect("case branch snapshots should restore the owner after each loan");
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
fn rejects_ins_loan_escape_through_owner_initialization_and_return() {
    let initialized = analyze_source("verb broken(ins input: Buffer) { erg stored = input; }")
        .expect_err("an ins loan must not initialize an escaping owner");
    assert!(matches!(
        initialized.kind,
        SemanticErrorKind::EscapingLoan { ref name } if name == "input"
    ));

    let returned = analyze_source("verb broken(ins input: Buffer) -> Buffer { return input; }")
        .expect_err("an ins loan must not escape through a return");
    assert!(matches!(
        returned.kind,
        SemanticErrorKind::EscapingLoan { ref name } if name == "input"
    ));
    assert_eq!(actus::diagnostics::semantic_diagnostic(&returned).code(), "E1082");
}

#[test]
fn rejects_ins_loan_as_owner_or_dat_transfer_argument() {
    let owner = analyze_source(
        "verb accept(erg input: Buffer) { } verb main() { erg buffer = Buffer[1]; accept(input: ins buffer); }",
    )
    .expect_err("an ins loan must not become an erg argument");
    assert!(matches!(
        owner.kind,
        SemanticErrorKind::InvalidArgumentRole { parameter, .. } if parameter == "input"
    ));

    let transfer = analyze_source(
        "verb consume(dat input: Buffer) { } verb main(ins buffer: Buffer) { consume(input: buffer); }",
    )
    .expect_err("an ins loan must not become a dat transfer");
    assert!(matches!(
        transfer.kind,
        SemanticErrorKind::EscapingLoan { name } if name == "buffer"
    ));

    let field_transfer = analyze_source(
        "struct Holder { erg payload: Buffer, } verb consume(dat input: Buffer) { } verb main(ins holder: Holder) { consume(input: holder.payload); }",
    )
    .expect_err("an ins loan field must not become a dat transfer");
    assert!(matches!(
        field_transfer.kind,
        SemanticErrorKind::EscapingLoan { name } if name == "holder"
    ));
}

#[test]
fn assigns_deterministic_loan_ids_and_resumes_each_call_once() {
    let model = analyze_source(
        "verb mutate(ins buffer: Buffer) { } verb main() { erg first = Buffer[1]; erg second = Buffer[1]; mutate(buffer: ins first); mutate(buffer: ins second); inspect(view: abs first); inspect(view: abs second); }",
    )
    .expect("sequential exclusive calls should pass");
    assert_eq!(model.exclusive_loans.iter().map(|loan| loan.id).collect::<Vec<_>>(), vec![0, 1]);
    assert!(model.bindings.iter().skip(1).all(|binding| {
        binding.ownership == OwnershipState::Active && binding.access == AccessState::Mutable
    }));
}

use actus::ast::Role;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{BindingState, CleanupAction, LoopExitKind, SemanticErrorKind, analyze};

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
    assert!(
        matches!(error.kind, SemanticErrorKind::MoveFrozen { name, borrow_ids } if name == "buffer" && borrow_ids == vec![0])
    );
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

#[test]
fn moves_returned_owners_out_of_the_function() {
    let model = analyze_source("verb create() -> Buffer { erg buffer = make(); return buffer; }")
        .expect("returning an owner should transfer ownership");
    assert!(matches!(model.bindings[0].state, BindingState::Moved));
}

#[test]
fn rejects_borrowed_returns() {
    let returned_binding = analyze_source(
        "verb broken(erg buffer: Buffer) -> Buffer { abs view = ref buffer; return view; }",
    )
    .expect_err("an abs binding must not escape through return");
    assert!(
        matches!(returned_binding.kind, SemanticErrorKind::BorrowedReturn { name } if name == "view")
    );

    let returned_expression =
        analyze_source("verb broken(erg buffer: Buffer) -> Buffer { return ref buffer; }")
            .expect_err("a temporary borrow must not escape through return");
    assert!(matches!(returned_expression.kind, SemanticErrorKind::BorrowedReturn { .. }));
}

#[test]
fn transfers_ownership_when_initializing_an_erg_binding() {
    let error = analyze_source(
        "verb broken() { erg source = make(); erg target = source; inspect(source); }",
    )
    .expect_err("the initializer must move the source owner");
    assert!(matches!(error.kind, SemanticErrorKind::UseAfterMove { name } if name == "source"));

    let error = analyze_source(
        "verb broken(erg source: Buffer) { abs view = ref source; erg target = source; }",
    )
    .expect_err("a frozen owner must not initialize another owner");
    assert!(matches!(error.kind, SemanticErrorKind::MoveFrozen { name, .. } if name == "source"));
}

#[test]
fn rejects_mutation_while_an_owner_is_frozen() {
    let error = analyze_source(
        "verb broken(erg buffer: Buffer) { abs view = ref buffer; buffer = make(); }",
    )
    .expect_err("frozen owners must not be mutated");
    assert!(matches!(error.kind, SemanticErrorKind::MoveFrozen { name, .. } if name == "buffer"));
}

#[test]
fn passes_shared_borrows_to_known_nested_calls() {
    analyze_source(
        "verb inspect_view(abs view: Buffer) { inspect(view); } verb caller(erg buffer: Buffer) { abs view = ref buffer; inspect_view(view); }",
    )
    .expect("an active shared borrow should pass to an abs parameter");
}

#[test]
fn rejects_borrow_storage_in_a_longer_lived_owner() {
    let error = analyze_source(
        "verb broken(erg buffer: Buffer) { abs view = ref buffer; erg stored = view; }",
    )
    .expect_err("an abs binding must not initialize a longer-lived owner");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidOwnerInitializer { name } if name == "view")
    );
}

#[test]
fn plans_lifo_cleanup_and_skips_moved_bindings() {
    let model = analyze_source(
        "verb cleanup() { erg first = make(); erg second = make(); erg moved = make(); erg target = moved; drop(second); }",
    )
    .expect("cleanup plan should be generated");
    let actions = &model.cleanup_plans.last().expect("verb cleanup plan").actions;
    assert_eq!(
        actions,
        &[
            CleanupAction::DropBinding { binding_index: 3 },
            CleanupAction::DropBinding { binding_index: 0 },
        ]
    );
}

#[test]
fn plans_borrow_end_before_outer_owner_drop() {
    let model = analyze_source(
        "verb cleanup(erg buffer: Buffer) { { abs view = ref buffer; inspect(view); } }",
    )
    .expect("nested cleanup plans should be generated");
    assert_eq!(model.cleanup_plans[0].actions, vec![CleanupAction::EndBorrow { borrow_id: 0 }]);
    assert_eq!(
        model.cleanup_plans[1].actions,
        vec![CleanupAction::DropBinding { binding_index: 0 }]
    );
}

#[test]
fn plans_return_unwinding_without_dropping_the_returned_owner() {
    let model =
        analyze_source("verb early(erg first: Buffer, erg second: Buffer) { return first; }")
            .expect("return unwind plan should be generated");
    assert_eq!(model.return_unwind_plans.len(), 1);
    assert_eq!(
        model.return_unwind_plans[0].scopes[0].actions,
        vec![CleanupAction::DropBinding { binding_index: 1 }]
    );
}

#[test]
fn plans_return_unwinding_for_literal_returns() {
    let semantic = analyze_source("verb main() -> Int { return 42; }")
        .expect("literal return should pass semantic analysis");

    assert_eq!(semantic.return_unwind_plans.len(), 1);
    assert!(semantic.return_unwind_plans[0].span.end > semantic.return_unwind_plans[0].span.start);
}

#[test]
fn plans_return_unwinding_for_call_returns() {
    let semantic = analyze_source(
        "verb helper() -> Int { return 1; } verb main() -> Int { return helper(); }",
    )
    .expect("call return should pass semantic analysis");

    assert_eq!(semantic.return_unwind_plans.len(), 2);
}

#[test]
fn unwinds_nested_scopes_on_return() {
    let model =
        analyze_source("verb early(erg outer: Buffer) { { erg inner = make(); return outer; } }")
            .expect("nested return unwind plan should be generated");
    let scopes = &model.return_unwind_plans[0].scopes;
    assert_eq!(scopes[0].actions, vec![CleanupAction::DropBinding { binding_index: 1 }]);
    assert!(scopes[1].actions.is_empty());
}

#[test]
fn plans_break_and_continue_unwinding() {
    let model = analyze_source(
        "verb controls() { loop { erg local = make(); break; } loop { erg next = make(); continue; } }",
    )
    .expect("loop exits should generate cleanup plans");
    assert_eq!(model.loop_unwind_plans[0].kind, LoopExitKind::Break);
    assert_eq!(
        model.loop_unwind_plans[0].scopes[0].actions,
        vec![CleanupAction::DropBinding { binding_index: 0 }]
    );
    assert_eq!(model.loop_unwind_plans[1].kind, LoopExitKind::Continue);
    assert_eq!(
        model.loop_unwind_plans[1].scopes[0].actions,
        vec![CleanupAction::DropBinding { binding_index: 1 }]
    );
}

#[test]
fn rejects_loop_control_outside_a_loop() {
    let break_error =
        analyze_source("verb broken() { break; }").expect_err("break outside a loop must fail");
    assert!(
        matches!(break_error.kind, SemanticErrorKind::LoopControlOutsideLoop { keyword } if keyword == "break")
    );

    let continue_error = analyze_source("verb broken() { continue; }")
        .expect_err("continue outside a loop must fail");
    assert!(
        matches!(continue_error.kind, SemanticErrorKind::LoopControlOutsideLoop { keyword } if keyword == "continue")
    );
}

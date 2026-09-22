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
fn registers_structs_and_validates_owned_literals() {
    let model = analyze_source(
        "struct Point { x: Int, y: Int, } verb main() { erg point = Point { x: 1, y: 2, }; inspect(point.x); }",
    )
    .expect("valid struct declaration and literal should pass");

    assert_eq!(model.bindings.len(), 1);
}

#[test]
fn registers_enums_and_validates_unit_and_payload_construction() {
    analyze_source(
        "enum Color { Red, Green, } enum Message { Move(Int, Int), Write { text: String, }, Flag(Bool), } verb main(erg flag: Bool) { erg color: Color = Color.Red; erg move: Message = Message.Move(1, 2); erg write: Message = Message.Write(text: \"ok\"); erg flag_message: Message = Message.Flag(flag); }",
    )
    .expect("valid enum declarations and constructions should pass");
}

#[test]
fn rejects_invalid_enum_variants_and_payload_arguments() {
    let unknown_variant =
        analyze_source("enum Color { Red, } verb main() { erg color: Color = Color.Blue; }")
            .expect_err("unknown enum variants must fail");
    assert!(matches!(
        unknown_variant.kind,
        SemanticErrorKind::UnknownEnumVariant { enum_name, variant }
            if enum_name == "Color" && variant == "Blue"
    ));

    let wrong_count = analyze_source(
        "enum Message { Move(Int, Int), } verb main() { erg message: Message = Message.Move(1); }",
    )
    .expect_err("wrong payload arity must fail");
    assert!(matches!(
        wrong_count.kind,
        SemanticErrorKind::EnumVariantArgumentCount { expected: 2, found: 1, .. }
    ));

    let wrong_type = analyze_source(
        "enum Message { Move(Int), } verb main() { erg message: Message = Message.Move(\"wrong\"); }",
    )
    .expect_err("wrong payload types must fail");
    assert!(matches!(
        wrong_type.kind,
        SemanticErrorKind::EnumVariantArgumentTypeMismatch { expected, found, .. }
            if expected == "Int" && found == "String"
    ));

    let wrong_name = analyze_source(
        "enum Message { Write { text: String, }, } verb main() { erg message: Message = Message.Write(body: \"wrong\"); }",
    )
    .expect_err("unknown named payload fields must fail");
    assert!(matches!(
        wrong_name.kind,
        SemanticErrorKind::EnumVariantArgumentName { name, .. } if name == "body"
    ));
}

#[test]
fn rejects_unknown_and_recursive_enum_payload_types() {
    let unknown_type = analyze_source("enum Message { Invalid(Missing), }")
        .expect_err("unknown enum payload types must fail");
    assert!(matches!(
        unknown_type.kind,
        SemanticErrorKind::UnknownType { name } if name == "Missing"
    ));

    let recursive = analyze_source("enum Node { Next(Node), }")
        .expect_err("direct recursive enum payloads must fail");
    assert!(matches!(
        recursive.kind,
        SemanticErrorKind::RecursiveType { name } if name == "Node"
    ));
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
fn moves_grouped_owners_out_of_the_function_scope() {
    let model = analyze_source("verb create() -> Buffer { erg buffer = make(); return (buffer); }")
        .expect("grouped owner return should transfer ownership");
    assert_eq!(model.bindings[0].state, BindingState::Moved);
    assert!(
        model.return_unwind_plans[0].scopes[0]
            .actions
            .iter()
            .all(|action| !matches!(action, CleanupAction::DropBinding { binding_index: 0 }))
    );
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
fn rejects_borrows_hidden_by_expression_grouping() {
    let error = analyze_source("verb main() { erg value = make(); return (ref value); }")
        .expect_err("grouped borrow must not escape");

    assert!(matches!(error.kind, SemanticErrorKind::BorrowedReturn { .. }));
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

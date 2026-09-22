use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
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
fn validates_exhaustive_enum_patterns_and_payload_bindings() {
    analyze_source(
        "enum Color { Red, Green, } enum Message { Move(Int, Int), Write { text: String, }, } verb choose(erg color: Color) -> Int { return case color { Color.Red => 1, Color.Green => 2, }; } verb read(erg message: Message) -> Int { return case message { Message.Move(x, y) => x + y, Message.Write(text: text) => print(text), }; }",
    )
    .expect("complete enum patterns and typed bindings should pass");
}

#[test]
fn rejects_non_exhaustive_enum_and_primitive_patterns() {
    let enum_error = analyze_source(
        "enum Color { Red, Green, } verb choose(erg color: Color) -> Int { return case color { Color.Red => 1, }; }",
    )
    .expect_err("enum matches must cover every variant");
    assert!(matches!(
        enum_error.kind,
        SemanticErrorKind::NonExhaustiveMatch { subject, missing }
            if subject == "Color" && missing == vec!["Green"]
    ));

    let primitive_error =
        analyze_source("verb choose(erg value: Int) -> Int { return case value { 1 => 1, }; }")
            .expect_err("primitive matches require a wildcard");
    assert!(matches!(
        primitive_error.kind,
        SemanticErrorKind::NonExhaustiveMatch { subject, .. } if subject == "primitive"
    ));
}

#[test]
fn rejects_duplicate_and_unreachable_patterns() {
    let duplicate = analyze_source(
        "enum Color { Red, Green, } verb choose(erg color: Color) -> Int { return case color { Color.Red => 1, Color.Red => 2, _ => 0, }; }",
    )
    .expect_err("duplicate variants must fail");
    assert!(matches!(
        duplicate.kind,
        SemanticErrorKind::DuplicatePattern { pattern } if pattern == "Color.Red"
    ));

    let unreachable = analyze_source(
        "enum Color { Red, } verb choose(erg color: Color) -> Int { return case color { _ => 0, Color.Red => 1, }; }",
    )
    .expect_err("patterns after a wildcard must fail");
    assert!(matches!(
        unreachable.kind,
        SemanticErrorKind::UnreachablePattern { pattern } if pattern == "Color.Red"
    ));
}

#[test]
fn keeps_abs_case_subjects_read_only_and_restores_the_owner() {
    let mutation = analyze_source(
        "enum Message { Move(Int), } verb broken(erg message: Message) { case abs message { Message.Move(x) => { x = 2; }, }; }",
    )
    .expect_err("abs case payloads must not be mutable");
    assert!(matches!(
        mutation.kind,
        SemanticErrorKind::InvalidMutation { name } if name == "x"
    ));

    let model = analyze_source(
        "enum Color { Red, } verb valid(erg color: Color) { case abs color { Color.Red => 0, }; color = Color.Red; }",
    )
    .expect("the owner should become active after an abs case");
    assert_eq!(model.bindings[0].state, actus::semantic::BindingState::Active);
}

#[test]
fn consumes_dat_case_subjects_and_rejects_abs_subjects() {
    let moved = analyze_source(
        "enum Color { Red, } verb broken(erg color: Color) { case dat color { Color.Red => 0, }; color = Color.Red; }",
    )
    .expect_err("dat case subjects must be moved after matching");
    assert!(matches!(
        moved.kind,
        SemanticErrorKind::UseAfterMove { name } if name == "color"
    ));

    let invalid = analyze_source(
        "enum Color { Red, } verb broken(abs color: Color) { case dat color { Color.Red => 0, }; }",
    )
    .expect_err("dat matching must require ownership");
    assert!(matches!(
        invalid.kind,
        SemanticErrorKind::InvalidCaseRole { mode, subject }
            if mode == "dat" && subject == "color"
    ));
}

#[test]
fn gives_dat_payload_bindings_owned_cleanup() {
    let model = analyze_source(
        "enum Message { Move(Int, Int), } verb consume(erg message: Message) { case dat message { Message.Move(x, y) => { drop(x); drop(y); }, }; }",
    )
    .expect("dat payload bindings should be owned in the branch");
    assert_eq!(model.bindings[1].role, actus::ast::Role::Dat);
    assert_eq!(model.bindings[2].role, actus::ast::Role::Dat);
    assert!(model.cleanup_plans.iter().all(|plan| {
        plan.actions.iter().all(|action| {
            !matches!(action, actus::semantic::CleanupAction::DropBinding { binding_index: 1 | 2 })
        })
    }));
}

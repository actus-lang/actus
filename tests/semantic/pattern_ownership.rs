use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{CleanupAction, OwnershipState, SemanticModel, analyze};

fn analyze_source(source: &str) -> Result<SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program)
}

#[test]
fn nested_dat_cases_keep_payload_cleanup_branch_local() {
    let model = analyze_source(
        "enum Inner { Data(Buffer), Empty, } enum Outer { Wrapped(Inner), Empty, } verb consume(erg outer: Outer) { case dat outer { Outer.Wrapped(inner) => { case dat inner { Inner.Data(payload) => { drop(payload); }, Inner.Empty => 0, }; }, Outer.Empty => 0, }; }",
    )
    .expect("nested dat cases should preserve ownership and cleanup");

    let outer = model.bindings.iter().find(|binding| binding.name == "outer").unwrap();
    assert_eq!(outer.ownership, OwnershipState::Moved);
    assert!(model.cleanup_plans.iter().all(|plan| {
        plan.actions
            .iter()
            .filter(|action| matches!(action, CleanupAction::DropBinding { .. }))
            .count()
            <= 1
    }));
}

#[test]
fn nested_abs_cases_restore_the_outer_owner_after_branch_join() {
    let model = analyze_source(
        "enum Inner { Ready, } enum Outer { Wrapped(Inner), Empty, } verb inspect(erg outer: Outer) { case abs outer { Outer.Wrapped(inner) => { case abs inner { Inner.Ready => 1, }; }, Outer.Empty => 0, }; outer = Outer.Empty; }",
    )
    .expect("nested abs cases should restore the owner after all branches");

    let outer = model.bindings.iter().find(|binding| binding.name == "outer").unwrap();
    assert_eq!(outer.ownership, OwnershipState::Active);
}

#[test]
fn dat_case_return_unwinds_remaining_payload_once() {
    let model = analyze_source(
        "enum Message { Data(Buffer), Empty, } verb consume(erg message: Message) { case dat message { Message.Data(payload) => { return; }, Message.Empty => { return; }, }; }",
    )
    .expect("case return should produce a valid cleanup plan");

    let payload_index =
        model.bindings.iter().position(|binding| binding.name == "payload").unwrap();
    let drops = model
        .return_unwind_plans
        .iter()
        .flat_map(|plan| plan.scopes.iter())
        .flat_map(|scope| scope.actions.iter())
        .filter(|action| matches!(action, CleanupAction::DropBinding { binding_index } if *binding_index == payload_index))
        .count();
    assert_eq!(drops, 1);
}

#[test]
fn isolates_ownership_transfers_between_abs_case_branches() {
    analyze_source(
        "enum Color { Red, Green, } verb inspect(erg color: Color, erg payload: Buffer) { case abs color { Color.Red => { erg first = payload; }, Color.Green => { erg second = payload; }, }; }",
    )
    .expect("each alternative branch should start with the same payload owner");
}

#[test]
fn isolates_drops_between_abs_case_branches() {
    let model = analyze_source(
        "enum Color { Red, Green, } verb inspect(erg color: Color, erg payload: Buffer) { case abs color { Color.Red => { drop(payload); }, Color.Green => { drop(payload); }, }; }",
    )
    .expect("each alternative branch should observe an active payload owner");
    let payload = model
        .bindings
        .iter()
        .find(|binding| binding.name == "payload")
        .expect("payload binding should exist");
    assert_eq!(payload.ownership, OwnershipState::Active);
}

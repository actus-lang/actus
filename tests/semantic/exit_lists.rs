//! The analyzer's exit lists (ADR-0125): each exit releases exactly the owners
//! live there, every list belongs to the body it was planned in, an `if`
//! branch is a scope of its own, the value of a branch moves out of it, an
//! assignment releases the value it replaces, and the right operand of `&&`
//! and `||` is a branch whose ownership must agree with the path that skips
//! it (`E1061`).
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{CleanupAction, SemanticError, SemanticErrorKind, SemanticModel, analyze};

const PRELUDE: &str = "verb eat(dat packet: Buffer) -> Bool { drop(packet); return true; } \
verb grow(dat packet: Buffer) -> Buffer { return packet; } enum Slot { Held(Buffer), Free, } ";

fn analyze_main(body: &str) -> Result<SemanticModel, SemanticError> {
    let source = format!("{PRELUDE}verb main() {{ erg n: Int = 1; {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

fn binding(model: &SemanticModel, name: &str) -> usize {
    model.bindings.iter().rposition(|binding| binding.name == name).expect("binding exists")
}

fn releases(actions: &[CleanupAction], index: usize) -> bool {
    actions.iter().any(|action| {
        matches!(action, CleanupAction::DropBinding { binding_index } if *binding_index == index)
    })
}

#[test]
fn a_conditional_operand_that_moves_an_owner_is_rejected() {
    for (label, body) in [
        ("and", "erg b: Buffer = Buffer[1]; if n > 0 && eat(packet: dat b) { print(1); }"),
        ("or", "erg b: Buffer = Buffer[1]; erg ok = n > 0 || eat(packet: dat b);"),
    ] {
        let error = analyze_main(body).expect_err(label);
        assert!(
            matches!(&error.kind, SemanticErrorKind::BranchStateMismatch { name, .. } if name == "b"),
            "{label}: {error:?}"
        );
    }
    analyze_main("erg b: Buffer = Buffer[1]; erg ok = n > 0 && n < 4; drop(b);")
        .expect("a conditional operand that moves nothing is valid");
}

#[test]
fn an_owner_declared_in_an_if_branch_is_released_by_the_branch() {
    let model =
        analyze_main("if n > 0 { erg b: Buffer = Buffer[1]; } else { erg b: Buffer = Buffer[2]; }")
            .expect("each branch is a scope, so the same name may be declared in both");
    let then_owner = binding(&model, "b") - 1;
    let else_owner = binding(&model, "b");
    let verb = model.verb_cleanups.iter().find(|verb| verb.name == "main").expect("main plans");
    let scopes = &model.cleanup_plans[verb.scopes.clone()];
    let body = scopes.last().expect("the body scope ends last");
    assert!(!releases(&body.actions, then_owner) && !releases(&body.actions, else_owner));
    assert_eq!(scopes.iter().filter(|scope| releases(&scope.actions, then_owner)).count(), 1);
    assert_eq!(scopes.iter().filter(|scope| releases(&scope.actions, else_owner)).count(), 1);
}

#[test]
fn the_value_of_a_branch_moves_out_before_the_branch_releases_its_owners() {
    let model = analyze_main(
        "erg kept: Buffer = if n > 0 { erg b: Buffer = Buffer[1]; b } else { Buffer[2] }; drop(kept);",
    )
    .expect("the branch value moves into the owner");
    let moved = binding(&model, "b");
    assert!(model.cleanup_plans.iter().all(|scope| !releases(&scope.actions, moved)));
}

#[test]
fn an_assignment_releases_the_replaced_value_unless_the_value_consumed_it() {
    let model = analyze_main(
        "erg b: Buffer = Buffer[1]; b = Buffer[2]; b = grow(packet: dat b); append(b, 1u8);",
    )
    .expect("an owner given back its consumed value is live again");
    let owner = binding(&model, "b");
    assert_eq!(model.replacement_drops.len(), 1);
    assert_eq!(model.replacement_drops[0].binding_index, owner);
    let verb = model.verb_cleanups.iter().find(|verb| verb.name == "main").expect("main plans");
    assert!(releases(&model.cleanup_plans[verb.scopes.end - 1].actions, owner));
}

#[test]
fn a_wildcard_arm_of_case_dat_releases_the_whole_subject() {
    let model = analyze_main(
        "erg b: Buffer = Buffer[1]; erg held: Slot = Slot.Held(b); \
erg k: Int = case dat held { Slot.Free => 1, _ => 2, };",
    )
    .expect("a wildcard arm consumes the subject");
    let subject = binding(&model, "held");
    assert!(model.cleanup_plans.iter().any(|scope| {
        scope.actions.iter().any(|action| {
            matches!(action, CleanupAction::DropPayloadField { binding_index, variant, .. }
                if *binding_index == subject && variant == "_")
        })
    }));
}

#[test]
fn every_exit_list_belongs_to_the_body_that_planned_it() {
    let model = analyze_main("erg b: Buffer = Buffer[1]; if n > 0 { return; } drop(b);")
        .expect("valid early return");
    let main = model.verb_cleanups.iter().find(|verb| verb.name == "main").expect("main plans");
    let eat = model.verb_cleanups.iter().find(|verb| verb.name == "eat").expect("eat plans");
    assert!(eat.returns.end <= main.returns.start);
    let early = &model.return_unwind_plans[main.returns.start];
    let owner = binding(&model, "b");
    assert!(early.scopes.iter().any(|scope| releases(&scope.actions, owner)));
}

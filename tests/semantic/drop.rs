use actus::ast::Role;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{CleanupAction, SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn registers_drop_contract_and_plans_owned_cleanup() {
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { print(self.value); } } verb main() -> Int { erg counter = Counter { value: 7, }; return 0; }";
    let model = analyze_source(source).expect("drop contract should be valid");
    assert_eq!(model.drop_types, vec!["Counter"]);
    assert!(model.cleanup_plans.iter().any(|plan| {
        plan.actions.iter().any(|action| matches!(action, CleanupAction::DropBinding { binding_index } if *binding_index == 0))
    }));
}

#[test]
fn rejects_invalid_drop_contract() {
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(abs self: Counter) { } }";
    let error = analyze_source(source).expect_err("Drop must require an exclusive receiver");
    assert!(
        matches!(error.kind, SemanticErrorKind::RoleMethodMismatch { role, method } if role == "Drop" && method == "drop")
    );
}

#[test]
fn rejects_drop_while_owner_is_frozen_or_exclusively_loaned() {
    let frozen = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } verb main() -> Int { erg counter = Counter { value: 7, }; { abs view = ref counter; drop(counter); } return 0; }";
    let error = analyze_source(frozen).expect_err("a frozen owner must not be dropped");
    assert!(matches!(error.kind, SemanticErrorKind::DropFrozen { name, .. } if name == "counter"));

    let loaned = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } verb use(ins counter: Counter) { drop(counter); }";
    let error = analyze_source(loaned).expect_err("an ins loan must not be dropped");
    assert!(matches!(error.kind, SemanticErrorKind::DropBorrow { name } if name == "counter"));
}

#[test]
fn moved_owner_is_removed_from_drop_plan() {
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } verb consume(dat value: Counter) { } verb main() -> Int { erg counter = Counter { value: 7, }; consume(value: counter); return 0; }";
    let model = analyze_source(source).expect("moved owner should be valid");
    let counter_index =
        model.bindings.iter().position(|binding| binding.name == "counter").unwrap();
    assert!(model.cleanup_plans.iter().all(|plan| {
        plan.actions.iter().all(|action| !matches!(action, CleanupAction::DropBinding { binding_index } if *binding_index == counter_index))
    }));
}

#[test]
fn drop_receiver_uses_the_instrumental_role() {
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } verb main() -> Int { erg counter = Counter { value: 7, }; return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let method = match &program.declarations[1] {
        actus::ast::TopLevelDecl::Perform(perform) => &perform.methods[0],
        _ => panic!("expected Drop performance"),
    };
    assert_eq!(method.params[0].role, Role::Ins);
}

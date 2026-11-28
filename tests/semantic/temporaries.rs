//! Bug #128: a value that no binding receives is a temporary with a release of
//! its own (ADR-0125), and a `?` nested in an expression is bound, with the
//! values evaluated before it, to generated owners that its exit list
//! releases.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{CleanupAction, SemanticModel, analyze};

const PRELUDE: &str = "struct Tracker { id: Int, } \
perform Drop for Tracker { verb drop(ins self: Tracker) { print(self.id); } } \
struct Pair { erg a: Tracker, erg b: Tracker, } enum Fail { Bad, } \
verb pair() -> Pair { return Pair { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }; } \
verb make() -> Buffer { return Buffer[3]; } verb number() -> Int { return 1; } \
verb track(erg id: Int) -> Tracker { return Tracker { id: id, }; } \
verb tracked() -> Result[Tracker, Fail] { return Result[Tracker, Fail].Err(Fail.Bad); } ";

fn analyzed(body: &str) -> SemanticModel {
    let source = format!("{PRELUDE}{body}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse")).expect("source must be accepted")
}

#[test]
fn a_discarded_call_that_holds_a_resource_is_a_temporary() {
    let model = analyzed("verb main() { make(); pair(); number(); }");
    let names: Vec<_> =
        model.temporary_releases.iter().map(|temporary| temporary.type_name.name.clone()).collect();
    assert_eq!(names, ["Buffer", "Pair"]);
    assert!(
        model.temporary_releases.iter().all(|temporary| temporary.span == temporary.release_after)
    );
}

#[test]
fn a_call_read_through_a_field_is_released_after_the_outermost_read() {
    let model = analyzed("verb main() { erg n: Int = pair().a.id; }");
    let [temporary] = model.temporary_releases.as_slice() else {
        panic!("one temporary: {:?}", model.temporary_releases)
    };
    assert_eq!(temporary.type_name.name, "Pair");
    assert!(temporary.release_after.end > temporary.span.end);
}

#[test]
fn a_value_received_by_a_binding_or_a_block_is_not_a_temporary() {
    let model = analyzed(
        "verb main() { erg b: Buffer = make(); erg n: Int = 1; \
erg c: Buffer = if n > 0 { make() } else { make() }; } \
verb pick(erg n: Int) -> Buffer { return case n { 0 => { make(); }, _ => { make(); }, }; }",
    );
    assert!(model.temporary_releases.is_empty(), "{:?}", model.temporary_releases);
}

#[test]
fn a_nested_try_releases_the_operand_evaluated_before_it_when_it_fails() {
    let model = analyzed(
        "verb run() -> Result[Int, Fail] { erg one: Int = 1; \
erg p: Pair = Pair { a: track(id: one), b: tracked()?, }; return Result[Int, Fail].Ok(4); }",
    );
    let generated = model
        .bindings
        .iter()
        .position(|binding| binding.name.starts_with("__actus_generated_"))
        .expect("the operand before the `?` is bound to a generated owner");
    let releases = |plan: &actus::semantic::UnwindPlan| {
        plan.scopes.iter().flat_map(|scope| &scope.actions).any(|action| {
            matches!(action, CleanupAction::DropBinding { binding_index } if *binding_index == generated)
        })
    };
    assert!(model.return_unwind_plans.iter().any(releases), "{:?}", model.return_unwind_plans);
}

use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{CleanupAction, ScopeCleanup, SemanticModel, analyze};

const PRELUDE: &str = "struct Node { value: Int, } struct PlainArena { held: Arena[64], } \
struct OwnedArena { erg held: Arena[64], } \
verb take(dat held: Arena[64]) {} verb lend(ins held: Arena[64]) {} verb view(abs held: Arena[64]) {} \
verb pass(dat held: Arena[64]) -> Arena[64] { return held; } ";

fn analyze_main(body: &str) -> SemanticModel {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse")).expect("program should be accepted")
}

fn binding(model: &SemanticModel, name: &str) -> usize {
    model.bindings.iter().position(|binding| binding.name == name).expect("binding should exist")
}

fn resets(scopes: &[ScopeCleanup], binding_index: usize) -> usize {
    scopes
        .iter()
        .flat_map(|scope| &scope.actions)
        .filter(|action| matches!(action, CleanupAction::ResetArena { binding_index: index } if *index == binding_index))
        .count()
}

fn scope_resets(model: &SemanticModel, name: &str) -> usize {
    resets(&model.cleanup_plans, binding(model, name))
}

fn return_resets(model: &SemanticModel, name: &str) -> usize {
    let index = binding(model, name);
    model.return_unwind_plans.iter().map(|unwind| resets(&unwind.scopes, index)).sum()
}

#[test]
fn resets_a_live_arena_owner_exactly_once() {
    let model = analyze_main(
        "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); print(node.value);",
    );
    assert_eq!(scope_resets(&model, "arena"), 1);
}

#[test]
fn resets_the_arena_on_the_return_path_while_it_is_still_owned() {
    let model = analyze_main(
        "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); return;",
    );
    assert_eq!(return_resets(&model, "arena"), 1);
    assert_eq!(scope_resets(&model, "arena"), 1);
}

#[test]
fn does_not_reset_an_arena_that_moved_into_a_call() {
    let model = analyze_main("erg arena: Arena[64] = Arena[64](); take(held: arena);");
    assert_eq!(scope_resets(&model, "arena"), 0);
}

#[test]
fn does_not_reset_an_arena_that_moved_into_another_binding() {
    let model = analyze_main(
        "erg arena: Arena[64] = Arena[64](); erg other = arena; erg node = other.place(value: Node { value: 1, });",
    );
    assert_eq!(scope_resets(&model, "arena"), 0);
    assert_eq!(scope_resets(&model, "other"), 1);
}

#[test]
fn does_not_reset_an_arena_that_moved_into_a_struct_field() {
    for (label, holder) in [
        ("plain field", "PlainArena { held: arena, }"),
        ("erg field", "OwnedArena { held: arena, }"),
    ] {
        let model =
            analyze_main(&format!("erg arena: Arena[64] = Arena[64](); erg holder = {holder};"));
        assert_eq!(scope_resets(&model, "arena"), 0, "{label}");
    }
}

#[test]
fn does_not_reset_an_arena_that_was_dropped_or_returned_again() {
    let dropped = analyze_main("erg arena: Arena[64] = Arena[64](); drop(arena);");
    assert_eq!(scope_resets(&dropped, "arena"), 0);
    let passed = analyze_main(
        "erg arena: Arena[64] = Arena[64](); erg back: Arena[64] = pass(held: arena);",
    );
    assert_eq!(scope_resets(&passed, "arena"), 0);
    assert_eq!(scope_resets(&passed, "back"), 1);
}

#[test]
fn does_not_reset_an_arena_that_was_moved_on_the_return_path() {
    let model = analyze_main(
        "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); take(held: arena); return;",
    );
    assert_eq!(return_resets(&model, "arena"), 0);
    assert_eq!(scope_resets(&model, "arena"), 0);
}

#[test]
fn resets_the_arena_of_a_dat_parameter_but_not_of_a_loan_or_a_view() {
    // The parameters of `take`, `lend`, `view` and `pass` are all named `held`.
    let model = analyze_main("print(1);");
    let counts = model
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.name == "held")
        .map(|(index, candidate)| {
            (format!("{:?}", candidate.role), resets(&model.cleanup_plans, index))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        counts,
        vec![
            ("Dat".to_owned(), 1),
            ("Ins".to_owned(), 0),
            ("Abs".to_owned(), 0),
            ("Dat".to_owned(), 0),
        ],
        "only a dat parameter that is still owned at its scope end is reset; `pass` returns its parameter"
    );
}

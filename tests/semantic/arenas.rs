use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

#[test]
fn accepts_arena_capacity_and_place_with_local_provenance() {
    analyze_source(
        "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[4096] = Arena[4096](); erg node = arena.place(value: Node { value: 7, }); return node.value; }",
    )
    .expect("Arena capacity and place should be valid");
}

#[test]
fn rejects_zero_and_non_numeric_arena_capacity() {
    for source in [
        "verb main() { erg arena: Arena[0] = Arena[0](); }",
        "verb main() { erg arena: Arena[Bytes] = Arena[Bytes](); }",
    ] {
        let error = analyze_source(source).expect_err("invalid arena capacity must fail");
        assert!(matches!(error, SemanticErrorKind::InvalidArenaCapacity { .. }));
    }
}

#[test]
fn rejects_arena_derived_reference_escaping_through_return() {
    let error = analyze_source(
        "struct Node { value: Int, } verb leak() -> Node { erg arena: Arena[128] = Arena[128](); erg node = arena.place(value: Node { value: 1, }); return node; }",
    )
    .expect_err("arena-derived values must not escape their scope");
    assert!(matches!(error, SemanticErrorKind::ArenaReferenceEscape { name } if name == "node"));
}

#[test]
fn rejects_arena_reference_assignment_to_an_outer_scope() {
    let error = analyze_source(
        "struct Node { value: Int, } verb main() -> Int { erg outer = Node { value: 0, }; { erg arena: Arena[128] = Arena[128](); erg node = arena.place(value: Node { value: 1, }); outer = node; } return 0; }",
    )
    .expect_err("an arena-derived value must not enter an outer binding");
    assert!(matches!(error, SemanticErrorKind::ArenaReferenceEscape { name } if name == "outer"));
}

#[test]
fn rejects_arena_reference_hidden_in_a_returned_struct() {
    let error = analyze_source(
        "struct Node { value: Int, } struct Holder { inner: Node, } verb leak() -> Holder { erg arena: Arena[128] = Arena[128](); erg node = arena.place(value: Node { value: 1, }); return Holder { inner: node, }; }",
    )
    .expect_err("a struct must not hide an arena-derived value in a return");
    assert!(matches!(error, SemanticErrorKind::ArenaReferenceEscape { .. }));
}

#[test]
fn rejects_linking_values_from_two_distinct_arenas() {
    let error = analyze_source(
        "struct Node { value: Int, } verb main() -> Int { erg left: Arena[128] = Arena[128](); erg right: Arena[128] = Arena[128](); erg first = left.place(value: Node { value: 1, }); erg second = right.place(value: first); return 0; }",
    )
    .expect_err("values cannot be placed across arena provenance boundaries");
    assert!(matches!(error, SemanticErrorKind::CrossArenaReference { .. }));
}

#[test]
fn preserves_arena_provenance_through_ins_and_abs_bindings() {
    let model = analyze_source(
        "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[128] = Arena[128](); ins node = arena.place(value: Node { value: 1, }); abs view = ref node; return view.value; }",
    )
    .expect("ins and abs references should preserve arena provenance");
    let node = model.bindings.iter().position(|binding| binding.name == "node").unwrap();
    let view = model.bindings.iter().position(|binding| binding.name == "view").unwrap();
    assert_eq!(model.arena_provenance.get(&node), model.arena_provenance.get(&view));
}

const MOVE_PRELUDE: &str = "struct Node { value: Int, } struct Holder { erg held: Arena[64], } verb take(dat arena: Arena[64]) {} verb peek(ins arena: Arena[64]) {} verb look(abs arena: Arena[64]) {} ";

fn analyze_with_prelude(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    analyze_source(&format!("{MOVE_PRELUDE}verb main() {{ {body} }}"))
}

const PLACE_NODE: &str =
    "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); ";

#[test]
fn rejects_every_arena_move_while_a_view_is_live() {
    for (label, consume) in [
        ("dat argument", "take(arena: arena);"),
        ("owner binding", "erg other = arena;"),
        ("declared owner binding", "erg other: Arena[64] = arena;"),
        ("assignment", "erg other: Arena[64] = Arena[64](); other = arena;"),
        ("struct field", "erg holder = Holder { held: arena, };"),
        ("overwrite", "arena = Arena[64]();"),
        ("explicit drop", "drop(arena);"),
        ("nested block drop", "{ drop(arena); }"),
    ] {
        let error = analyze_with_prelude(&format!(
            "{PLACE_NODE}abs view = ref node; print(view.value); {consume}"
        ))
        .expect_err(label);
        assert!(
            matches!(&error, SemanticErrorKind::ArenaReferenceLive { arena, reference }
                if arena == "arena" && reference == "view"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn rejects_an_arena_move_while_an_ins_view_is_live() {
    let error = analyze_with_prelude(
        "erg arena: Arena[64] = Arena[64](); ins node = arena.place(value: Node { value: 1, }); abs view = ref node; take(arena: arena); print(view.value);",
    )
    .expect_err("an ins placement is a live view of the arena");
    assert!(
        matches!(error, SemanticErrorKind::ArenaReferenceLive { reference, .. } if reference == "node")
    );
}

#[test]
fn rejects_a_placed_value_read_after_its_arena_is_released() {
    for (label, consume) in [
        ("move", "take(arena: arena);"),
        ("drop", "drop(arena);"),
        ("block move", "{ erg other = arena; }"),
    ] {
        for read in ["print(node.value);", "abs again = ref node;", "erg copy = node;"] {
            let error =
                analyze_with_prelude(&format!("{PLACE_NODE}{consume} {read}")).expect_err(label);
            assert!(
                matches!(&error, SemanticErrorKind::ArenaReferenceReleased { arena, reference }
                    if arena == "arena" && reference == "node"),
                "{label} {read}: {error:?}"
            );
        }
    }
}

#[test]
fn rejects_the_reported_move_with_a_live_view_and_a_later_read() {
    let error = analyze_source(
        "struct Node { value: Int, } verb take(dat arena: Arena[64]) {} verb main() { erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); abs view = ref node; take(arena: arena); print(view.value); }",
    )
    .expect_err("the reported program must be rejected");
    assert!(matches!(error, SemanticErrorKind::ArenaReferenceLive { arena, reference }
        if arena == "arena" && reference == "view"));
}

#[test]
fn ends_a_view_with_the_block_that_holds_it() {
    for (label, consume) in [
        ("drop", "drop(arena);"),
        ("dat argument", "take(arena: arena);"),
        ("owner binding", "erg other = arena;"),
    ] {
        analyze_with_prelude(&format!(
            "{PLACE_NODE}{{ abs view = ref node; print(view.value); }} {consume}"
        ))
        .unwrap_or_else(|error| panic!("{label}: a closed block must not block: {error:?}"));
    }
}

#[test]
fn accepts_arena_uses_that_the_adr_allows() {
    for (label, body) in [
        (
            "loan and view while a view is live",
            "abs view = ref node; peek(arena: ins arena); look(arena: abs arena); print(view.value);",
        ),
        ("placed value read before the drop", "print(node.value); drop(arena);"),
        ("placed value kept by the new owner", "erg other = arena; print(node.value);"),
        ("unused placed value before a move", "take(arena: arena);"),
        ("view in a nested block", "{ { abs view = ref node; print(view.value); } } drop(arena);"),
        ("scope end release", "abs view = ref node; print(view.value);"),
    ] {
        analyze_with_prelude(&format!("{PLACE_NODE}{body}"))
            .unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

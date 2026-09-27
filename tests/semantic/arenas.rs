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

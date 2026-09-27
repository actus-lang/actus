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

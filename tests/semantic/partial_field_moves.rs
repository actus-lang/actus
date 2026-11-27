//! Moves are tracked per field path (ADR-0016, ADR-0046 2.2): moving one field
//! of an owner leaves its other fields usable, while the owner as a whole is
//! partially moved and cannot be used, moved or moved from again.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{CleanupAction, OwnershipState, SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Cfg { erg a: Buffer, erg b: Buffer, } \
struct Inner { erg a: Buffer, erg b: Buffer, } struct Outer { erg inner: Inner, erg z: Buffer, } \
verb sink(dat x: Buffer) { drop(x); } verb take(dat x: Cfg) { drop(x); } \
verb take_inner(dat x: Inner) { drop(x); } verb length(abs x: Buffer) -> Int { return 1; } \
verb length_cfg(abs x: Cfg) -> Int { return 1; } ";

fn analyze_body(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let source = format!("{PRELUDE}{body}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse")).map_err(|error| error.kind)
}

const CFG: &str = "erg c: Cfg = Cfg { a: Buffer[1], b: Buffer[1], }; ";
const OUTER: &str =
    "erg c: Outer = Outer { inner: Inner { a: Buffer[1], b: Buffer[1], }, z: Buffer[1], }; ";

#[test]
fn accepts_moving_disjoint_fields_of_one_owner() {
    for (label, body) in [
        ("two fields", format!("verb main() {{ {CFG} sink(x: dat c.a); sink(x: dat c.b); }}")),
        ("reverse order", format!("verb main() {{ {CFG} sink(x: dat c.b); sink(x: dat c.a); }}")),
        (
            "dat parameter owner",
            "verb f(dat c: Cfg) { sink(x: dat c.a); sink(x: dat c.b); }".to_owned(),
        ),
        (
            "nested siblings",
            format!(
                "verb main() {{ {OUTER} sink(x: dat c.inner.a); sink(x: dat c.inner.b); sink(x: dat c.z); }}"
            ),
        ),
        (
            "view of a field that stayed",
            format!("verb main() {{ {CFG} sink(x: dat c.a); erg n: Int = length(x: abs c.b); }}"),
        ),
    ] {
        analyze_body(&body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn keeps_the_owner_partially_moved_with_the_exact_paths() {
    let model = analyze_body(&format!(
        "verb main() {{ {CFG} sink(x: dat c.a); }} verb nested() {{ {OUTER} sink(x: dat c.inner.a); sink(x: dat c.z); }}"
    ))
    .expect("accepted");
    let owners: Vec<_> = model
        .bindings
        .iter()
        .filter(|binding| binding.name == "c")
        .map(|binding| binding.ownership.clone())
        .collect();
    assert_eq!(owners[0], OwnershipState::PartiallyMoved { fields: vec!["a".to_owned()] });
    assert_eq!(
        owners[1],
        OwnershipState::PartiallyMoved { fields: vec!["inner.a".to_owned(), "z".to_owned()] }
    );
}

#[test]
fn rejects_whole_use_and_repeated_moves_after_a_partial_move() {
    for (label, tail) in [
        ("same field twice", "sink(x: dat c.a);"),
        ("read the moved field", "erg n: Int = length(x: abs c.a);"),
        ("move the whole owner", "take(x: dat c);"),
        ("view the whole owner", "erg n: Int = length_cfg(x: abs c);"),
    ] {
        let error = analyze_body(&format!("verb main() {{ {CFG} sink(x: dat c.a); {tail} }}"))
            .expect_err(label);
        assert!(
            matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "c"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn rejects_overlapping_nested_paths() {
    for (label, first, second) in [
        ("parent after child", "sink(x: dat c.inner.a);", "take_inner(x: dat c.inner);"),
        ("child after parent", "take_inner(x: dat c.inner);", "sink(x: dat c.inner.a);"),
        ("same child twice", "sink(x: dat c.inner.a);", "sink(x: dat c.inner.a);"),
    ] {
        let error =
            analyze_body(&format!("verb main() {{ {OUTER} {first} {second} }}")).expect_err(label);
        assert!(matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "c"));
    }
}

#[test]
fn drops_a_partially_moved_owner_with_its_remaining_fields_only() {
    let model = analyze_body(&format!("verb main() {{ {CFG} sink(x: dat c.a); drop(c); }}"))
        .expect("dropping the rest of a partially moved owner is valid");
    let main_drop = model.explicit_drops.last().expect("an explicit drop");
    assert_eq!(main_drop.moved_fields, vec!["a".to_owned()]);
    assert!(model.cleanup_plans.iter().all(|plan| {
        plan.actions.iter().all(|action| {
            !matches!(action, CleanupAction::DropBinding { binding_index }
                if *binding_index == main_drop.binding_index)
        })
    }));
    let error =
        analyze_body(&format!("verb main() {{ {CFG} sink(x: dat c.a); drop(c); drop(c); }}"))
            .expect_err("a dropped owner cannot be dropped again");
    assert!(matches!(error, SemanticErrorKind::DoubleDrop { .. }));
}

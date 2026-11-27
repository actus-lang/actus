//! A view that a `case` takes of its subject ends with the case (ADR-0016): the
//! owner is released by its own binding, so a later verb that names its owner
//! the same way, an arm that views another field, and a second case do not
//! leave it frozen.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Cfg { erg a: Buffer, erg b: Option[Buffer], } \
verb one(abs x: Buffer) {} verb touch(dat c: Cfg) { drop(c); } ";

fn analyze_verbs(verbs: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let source = format!("{PRELUDE}{verbs} verb main() -> Int {{ return 0; }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse")).map_err(|error| error.kind)
}

#[test]
fn releases_the_owner_when_the_case_ends_although_an_arm_viewed_another_field() {
    for (label, mode, arm) in [
        ("plain subject, arm views another field", "", "one(x: abs c.a);"),
        ("abs subject, arm views another field", "abs ", "one(x: abs c.a);"),
        ("plain subject, empty arm", "", ""),
        ("abs subject, arm views the payload", "abs ", "one(x: abs inner);"),
    ] {
        let verbs = format!(
            "verb f(dat c: Cfg) {{ case {mode}c.b {{ Option.None => {{}}, \
             Option.Some(inner) => {{ {arm} }}, }}; drop(c); }}"
        );
        analyze_verbs(&verbs).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn releases_the_owner_of_the_current_verb_not_one_of_the_same_name_elsewhere() {
    let verbs = "verb f1(dat c: Cfg) { drop(c); } \
        verb f2(dat c: Cfg) { case c.b { Option.None => {}, \
        Option.Some(inner) => { one(x: abs c.a); }, }; drop(c); } \
        verb f3(dat c: Cfg) { case c.b { Option.None => {}, \
        Option.Some(inner) => { one(x: abs c.a); }, }; \
        case c.b { Option.None => {}, Option.Some(inner) => { }, }; drop(c); }";
    analyze_verbs(verbs).expect("every verb releases its own owner when its case ends");
}

#[test]
fn keeps_the_owner_frozen_inside_the_case() {
    for (label, arm) in [("drop", "drop(c);"), ("move", "touch(c: dat c);")] {
        let verbs = format!(
            "verb f(dat c: Cfg) {{ case c.b {{ Option.None => {{}}, \
             Option.Some(inner) => {{ {arm} }}, }}; }}"
        );
        let error = analyze_verbs(&verbs).expect_err(label);
        assert!(
            matches!(
                error,
                SemanticErrorKind::DropFrozen { .. } | SemanticErrorKind::MoveFrozen { .. }
            ),
            "{label}: {error:?}"
        );
    }
}

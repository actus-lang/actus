//! A view held by a value belongs to the place that holds the value
//! (ADR-0046 2.3, bug 106). A source that is released before that place, or
//! that is declared in a deeper scope than it, must be rejected; a source that
//! lives as long as the holder stays accepted.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Hv { abs b: Buffer, } struct Holder { erg kept: Hv, } ";

fn analyze_program(body: &str) -> Result<(), SemanticErrorKind> {
    let source = format!("{PRELUDE}{body}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map(|_| ()).map_err(|error| error.kind)
}

fn analyze_main(body: &str) -> Result<(), SemanticErrorKind> {
    analyze_program(&format!("verb main() {{ {body} }}"))
}

fn escaped_name(label: &str, result: Result<(), SemanticErrorKind>) -> String {
    match result {
        Err(SemanticErrorKind::BorrowedReturn { name }) => name,
        other => panic!("{label}: expected BorrowedReturn, found {other:?}"),
    }
}

#[test]
fn an_if_value_freezes_the_sources_of_every_arm() {
    let error = analyze_main(
        "erg c: Bool = true; erg local: Buffer = Buffer[4]; erg other: Buffer = Buffer[8]; \
         erg h = if c { Hv { b: local, } } else { Hv { b: other, } }; drop(local);",
    )
    .expect_err("the source of the then arm must stay frozen");
    assert!(matches!(error, SemanticErrorKind::DropFrozen { name, .. } if name == "local"));
}

#[test]
fn an_if_or_case_value_returned_holds_the_views_of_its_arms() {
    let result = analyze_program(
        "verb f(abs c: Bool) -> Hv { erg local: Buffer = Buffer[4]; erg other: Buffer = Buffer[8]; \
         return if c { Hv { b: local, } } else { Hv { b: other, } }; } verb main() {}",
    );
    assert_eq!(escaped_name("if", result), "local");
    let result = analyze_program(
        "verb f(abs n: Int) -> Hv { erg local: Buffer = Buffer[4]; \
         return case n { 0 => Hv { b: local, }, _ => Hv { b: local, }, }; } verb main() {}",
    );
    assert_eq!(escaped_name("case", result), "local");
}

#[test]
fn a_case_pattern_variable_holds_the_views_of_its_subject() {
    let result = analyze_program(
        "verb f(abs fallback: Buffer) -> Hv { erg local: Buffer = Buffer[4]; \
         erg opt = Option[abs Buffer].Some(local); \
         case opt { Option.Some(inner) => { return Hv { b: inner, }; }, Option.None => { }, }; \
         return Hv { b: fallback, }; } verb main() {}",
    );
    assert_eq!(escaped_name("abs payload", result), "local");
    let result = analyze_program(
        "verb f(abs fallback: Buffer) -> Hv { erg local: Buffer = Buffer[4]; \
         erg w = Option[Hv].Some(Hv { b: local, }); \
         case dat w { Option.Some(hv) => { return hv; }, Option.None => { }, }; \
         return Hv { b: fallback, }; } verb main() {}",
    );
    assert_eq!(escaped_name("dat payload", result), "local");
}

#[test]
fn a_case_pattern_variable_of_an_abs_parameter_derives_from_it() {
    analyze_program(
        "verb g(abs o: Option[abs Buffer], abs fallback: Buffer) -> Hv { \
         case o { Option.Some(inner) => { return Hv { b: inner, }; }, Option.None => { }, }; \
         return Hv { b: fallback, }; } verb main() {}",
    )
    .expect("a view of an abs parameter may be returned");
}

#[test]
fn a_view_of_an_inner_local_cannot_be_stored_in_an_outer_place() {
    for (label, body) in [
        (
            "binding",
            "erg keep: Buffer = Buffer[1]; erg h = Hv { b: keep, }; \
             { erg local: Buffer = Buffer[4]; h = Hv { b: local, }; }",
        ),
        (
            "binding from an if block",
            "erg c: Bool = true; erg keep: Buffer = Buffer[1]; erg h = Hv { b: keep, }; \
             if c { erg local: Buffer = Buffer[4]; h = Hv { b: local, }; }",
        ),
        (
            "field",
            "erg keep: Buffer = Buffer[1]; erg outer = Holder { kept: Hv { b: keep, }, }; \
             { erg local: Buffer = Buffer[4]; outer.kept = Hv { b: local, }; }",
        ),
        (
            "element",
            "erg xs: Array[Hv, 2] = Array[Hv, 2](); \
             { erg local: Buffer = Buffer[4]; xs[0] = Hv { b: local, }; }",
        ),
    ] {
        assert_eq!(escaped_name(label, analyze_main(body)), "local", "{label}");
    }
}

#[test]
fn a_view_stored_through_an_ins_element_must_come_from_an_abs_parameter() {
    let result = analyze_program(
        "verb f(ins out: Array[Hv, 2]) { erg local: Buffer = Buffer[4]; \
         out[0] = Hv { b: local, }; } verb main() {}",
    );
    assert_eq!(escaped_name("ins element", result), "local");
    analyze_program(
        "verb f(ins out: Array[Hv, 2], abs source: Buffer) { out[0] = Hv { b: source, }; } \
         verb main() {}",
    )
    .expect("a view of an abs parameter may be stored through an ins element");
}

#[test]
fn a_view_stored_in_an_element_freezes_its_source() {
    let error = analyze_main(
        "erg xs: Array[Hv, 2] = Array[Hv, 2](); erg local: Buffer = Buffer[4]; \
         xs[0] = Hv { b: local, }; drop(local);",
    )
    .expect_err("the source of a stored view must stay frozen");
    assert!(matches!(error, SemanticErrorKind::DropFrozen { name, .. } if name == "local"));
}

#[test]
fn a_source_that_lives_as_long_as_its_holder_is_accepted() {
    for (label, body) in [
        (
            "if value in the same scope",
            "erg c: Bool = true; erg local: Buffer = Buffer[4]; erg other: Buffer = Buffer[8]; \
             erg h = if c { Hv { b: local, } } else { Hv { b: other, } }; print(1);",
        ),
        (
            "holder in an inner block, then drop",
            "erg c: Bool = true; erg local: Buffer = Buffer[4]; erg other: Buffer = Buffer[8]; \
             { erg h = if c { Hv { b: local, } } else { Hv { b: other, } }; print(1); } \
             drop(local); drop(other);",
        ),
        (
            "assignment from the holder's scope",
            "erg keep: Buffer = Buffer[1]; erg keep2: Buffer = Buffer[2]; \
             erg h = Hv { b: keep, }; { h = Hv { b: keep2, }; } print(1);",
        ),
        (
            "assignment in a branch",
            "erg c: Bool = true; erg keep: Buffer = Buffer[1]; erg keep2: Buffer = Buffer[2]; \
             erg h = Hv { b: keep, }; if c { h = Hv { b: keep2, }; } print(1);",
        ),
        (
            "element from the holder's scope",
            "erg xs: Array[Hv, 2] = Array[Hv, 2](); erg local: Buffer = Buffer[4]; \
             { xs[0] = Hv { b: local, }; } print(1);",
        ),
        (
            "case dat value kept in the verb",
            "erg local: Buffer = Buffer[4]; erg w = Option[Hv].Some(Hv { b: local, }); \
             erg h = case dat w { Option.Some(hv) => hv, Option.None => { return; }, }; print(1);",
        ),
    ] {
        analyze_main(body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn a_branch_that_stored_a_view_keeps_the_source_frozen_after_it() {
    let error = analyze_main(
        "erg c: Bool = true; erg keep: Buffer = Buffer[1]; erg keep2: Buffer = Buffer[2]; \
         erg h = Hv { b: keep, }; if c { h = Hv { b: keep2, }; } drop(keep2);",
    )
    .expect_err("the source stays frozen while the holder lives");
    assert!(matches!(error, SemanticErrorKind::DropFrozen { name, .. } if name == "keep2"));
}

#[test]
fn a_branch_that_returns_does_not_keep_the_view_it_stored() {
    analyze_program(
        "verb f() -> Int { erg c: Bool = true; erg keep: Buffer = Buffer[1]; \
         erg keep2: Buffer = Buffer[2]; erg h = Hv { b: keep, }; \
         if c { h = Hv { b: keep2, }; return buffer_length(buffer: abs h.b); } \
         drop(keep2); return buffer_length(buffer: abs h.b); } verb main() {}",
    )
    .expect("the path after the if never stored the view");
}

#[test]
fn a_branch_that_continues_keeps_the_view_it_stored() {
    let error = analyze_main(
        "erg c: Bool = true; erg keep: Buffer = Buffer[1]; erg keep2: Buffer = Buffer[2]; \
         erg h = Hv { b: keep, }; if c { h = Hv { b: keep2, }; print(1); } drop(keep2);",
    )
    .expect_err("the view may have been stored");
    assert!(matches!(error, SemanticErrorKind::DropFrozen { name, .. } if name == "keep2"));
}

#[test]
fn a_case_arm_that_returns_does_not_keep_the_view_it_stored() {
    analyze_program(
        "verb f(abs n: Int) -> Int { erg keep: Buffer = Buffer[1]; erg keep2: Buffer = Buffer[2]; \
         erg h = Hv { b: keep, }; case n { 0 => { h = Hv { b: keep2, }; return buffer_length(buffer: abs h.b); }, _ => { }, }; \
         drop(keep2); return buffer_length(buffer: abs h.b); } verb main() {}",
    )
    .expect("the path after the case never stored the view");
}

#[test]
fn a_case_arm_that_joins_keeps_the_view_it_stored() {
    let result = analyze_program(
        "verb f(abs n: Int) -> Int { erg keep: Buffer = Buffer[1]; erg keep2: Buffer = Buffer[2]; \
         erg h = Hv { b: keep, }; case n { 0 => { h = Hv { b: keep2, }; }, _ => { }, }; \
         drop(keep2); return buffer_length(buffer: abs h.b); } verb main() {}",
    );
    assert!(
        matches!(result, Err(SemanticErrorKind::DropFrozen { ref name, .. }) if name == "keep2"),
        "{result:?}"
    );
}

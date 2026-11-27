//! The state after a loop is the join of the states at every `break` that
//! targets it (ADR-0016). A value that is moved or dropped before such a
//! `break` is moved after the loop, so a later use is rejected and the scope
//! end never cleans it up a second time. A `return` leaves the verb and a
//! `continue` goes to the back edge, so neither reaches the state after the
//! loop. Where the break paths disagree, the loop is rejected like an `if`
//! whose branches disagree.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticError, SemanticErrorKind, analyze};

const PRELUDE: &str = "verb eat(dat packet: Buffer) { drop(packet); } ";
const SOURCE: &str = "erg x: Buffer = Buffer[4]; erg n: Int = 0; ";

fn analyze_main(body: &str) -> Result<actus::semantic::SemanticModel, SemanticError> {
    let source = format!("{PRELUDE}verb main() {{ {SOURCE}{body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn a_move_before_the_only_break_is_a_move_after_the_loop() {
    for (label, body) in [
        (
            "the reported program",
            "loop { if n > 2 { eat(packet: x); break; } n += 1; } append(x, 1u8);",
        ),
        (
            "every break moves",
            "loop { if n > 2 { eat(packet: x); break; } if n > 5 { eat(packet: x); break; } n += 1; } append(x, 1u8);",
        ),
        ("a break in a block", "loop { n += 1; { eat(packet: x); break; } } append(x, 1u8);"),
        (
            "a break in a case arm",
            "loop { case n { 1 => { eat(packet: x); break; }, _ => { n += 1; }, }; } append(x, 1u8);",
        ),
        ("a second move", "loop { if n > 2 { eat(packet: x); break; } n += 1; } eat(packet: x);"),
    ] {
        let error = analyze_main(body).expect_err(&format!("{label}: x was moved on the break"));
        assert!(
            matches!(&error.kind, SemanticErrorKind::UseAfterMove { name } if name == "x"),
            "{label}: {:?}",
            error.kind
        );
    }
}

#[test]
fn a_drop_before_a_break_is_a_drop_after_the_loop() {
    let error = analyze_main("loop { if n > 2 { drop(x); break; } n += 1; } append(x, 1u8);")
        .expect_err("x was dropped on the break");
    assert!(
        matches!(&error.kind, SemanticErrorKind::UseAfterDrop { name } if name == "x"),
        "{:?}",
        error.kind
    );
}

#[test]
fn a_second_drop_after_the_loop_is_a_double_drop() {
    let error = analyze_main("loop { drop(x); break; } drop(x);").expect_err("x is dropped twice");
    assert!(matches!(&error.kind, SemanticErrorKind::DoubleDrop { name } if name == "x"));
}

#[test]
fn a_move_on_a_break_path_that_is_not_used_afterwards_is_accepted() {
    for (label, body) in [
        ("loop", "loop { if n > 2 { eat(packet: x); break; } n += 1; }"),
        (
            "every break moves",
            "loop { if n > 2 { eat(packet: x); break; } if n > 5 { eat(packet: x); break; } n += 1; }",
        ),
    ] {
        if let Err(error) = analyze_main(body) {
            panic!("{label}: a moved value that is not used again is valid: {:?}", error.kind);
        }
    }
}

#[test]
fn a_move_before_a_return_does_not_reach_the_state_after_the_loop() {
    for (label, body) in [
        (
            "loop",
            "loop { if n > 2 { eat(packet: x); return; } if n > 5 { break; } n += 1; } append(x, 1u8); drop(x);",
        ),
        (
            "for",
            "for erg i in 0..3 { if n > 1 { eat(packet: x); return; } n += 1; } append(x, 1u8); drop(x);",
        ),
    ] {
        if let Err(error) = analyze_main(body) {
            panic!("{label}: a return never reaches the code after the loop: {:?}", error.kind);
        }
    }
}

#[test]
fn a_break_belongs_to_its_innermost_loop_only() {
    // The inner break moves x, so the inner loop ends with x moved, and the
    // outer break then leaves with it moved.
    let error = analyze_main(
        "loop { loop { if n > 2 { eat(packet: x); break; } n += 1; } break; } append(x, 1u8);",
    )
    .expect_err("x is moved when the outer loop is left");
    assert!(matches!(&error.kind, SemanticErrorKind::UseAfterMove { name } if name == "x"));
    // The inner break moves a value of the outer body, which the outer loop
    // does not see after it; x is untouched on every path out of the outer one.
    for (label, body) in [
        (
            "inner value",
            "loop { erg y: Buffer = Buffer[4]; loop { eat(packet: y); break; } break; } append(x, 1u8); drop(x);",
        ),
        ("move after the inner loop", "loop { loop { break; } eat(packet: x); break; }"),
        (
            "inner break only",
            "loop { loop { n += 1; if n > 2 { break; } } eat(packet: x); break; }",
        ),
    ] {
        if let Err(error) = analyze_main(body) {
            panic!("{label}: the inner break does not move x: {:?}", error.kind);
        }
    }
    // A move after the inner loop is on the outer break path.
    let error = analyze_main("loop { loop { break; } eat(packet: x); break; } append(x, 1u8);")
        .expect_err("x is moved on the outer break");
    assert!(matches!(&error.kind, SemanticErrorKind::UseAfterMove { name } if name == "x"));
}

#[test]
fn break_paths_that_disagree_are_rejected_like_the_branches_of_an_if() {
    let error =
        analyze_main("loop { n += 1; if n > 2 { eat(packet: x); break; } if n > 5 { break; } }")
            .expect_err("x is moved on one break path and owned on the other");
    assert!(
        matches!(&error.kind, SemanticErrorKind::BranchStateMismatch { name, .. } if name == "x"),
        "{:?}",
        error.kind
    );
}

#[test]
fn a_loop_without_a_move_before_its_breaks_keeps_the_value() {
    for (label, body) in [
        ("loop", "loop { if n > 2 { break; } n += 1; } append(x, 1u8); drop(x);"),
        ("for", "for erg i in 0..3 { if n > 1 { break; } n += 1; } append(x, 1u8); drop(x);"),
        ("read before break", "loop { append(x, 1u8); break; } append(x, 2u8); drop(x);"),
    ] {
        if let Err(error) = analyze_main(body) {
            panic!("{label}: nothing was moved: {:?}", error.kind);
        }
    }
}

#[test]
fn the_normal_exit_of_a_for_loop_joins_the_break_paths() {
    for (label, body) in [
        ("move then break", "for erg i in 0..3 { if n > 1 { eat(packet: x); break; } n += 1; }"),
        ("drop then break", "for erg i in 0..3 { if n > 1 { drop(x); break; } n += 1; } "),
        ("unconditional", "for erg i in 0..3 { eat(packet: x); break; }"),
        (
            "a use after it",
            "for erg i in 0..3 { if n > 1 { eat(packet: x); break; } n += 1; } append(x, 1u8);",
        ),
        (
            "array",
            "erg a: Array[u32, 2] = Array[u32, 2](); for erg v: u32 in a { if n > 1 { eat(packet: x); break; } n += 1; }",
        ),
    ] {
        let error = analyze_main(body).expect_err(&format!("{label}: the exits disagree"));
        assert!(
            matches!(&error.kind, SemanticErrorKind::BranchStateMismatch { name, .. } if name == "x"),
            "{label}: {:?}",
            error.kind
        );
    }
}

#[test]
fn a_for_loop_whose_exits_all_keep_the_value_is_accepted() {
    for (label, body) in [
        ("break", "for erg i in 0..3 { if n > 1 { break; } n += 1; } append(x, 1u8); drop(x);"),
        ("no break", "for erg i in 0..3 { append(x, 1u8); } drop(x);"),
        (
            "array",
            "erg a: Array[u32, 2] = Array[u32, 2](); for erg v: u32 in a { if n > 1 { break; } n += 1; } drop(x);",
        ),
        (
            "nested",
            "for erg i in 0..3 { for erg j in 0..3 { if n > 1 { break; } n += 1; } } drop(x);",
        ),
        (
            "inner value",
            "for erg i in 0..3 { erg y: Buffer = Buffer[4]; if n > 1 { eat(packet: y); break; } drop(y); } drop(x);",
        ),
    ] {
        if let Err(error) = analyze_main(body) {
            panic!("{label}: every exit keeps x: {:?}", error.kind);
        }
    }
}

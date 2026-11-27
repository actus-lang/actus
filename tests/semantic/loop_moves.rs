//! A loop body is analysed once, so a value that is declared before the loop
//! and moved or dropped inside it is moved again by the next iteration
//! (ADR-0016). The compiler rejects the move unless every path that reaches
//! the next iteration leaves the value untouched, which is the case when the
//! path ends in `break` or `return` after the move.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticError, SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Holder { held: Buffer, } struct Owned { erg held: Buffer, } \
enum Slot { Full(Buffer), Empty, } verb eat(dat packet: Buffer) { drop(packet); } ";

fn analyze_main(body: &str) -> (String, Result<actus::semantic::SemanticModel, SemanticError>) {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    let result = analyze(&program);
    (source, result)
}

const SOURCE: &str = "erg x: Buffer = Buffer[4]; erg n: Int = 0; ";

#[test]
fn a_move_that_reaches_the_next_iteration_is_rejected_at_the_move() {
    for (label, body, moved_text) in [
        ("loop dat argument", "loop { if n > 2 { break; } eat(packet: x); n += 1; }", "x"),
        ("loop drop", "loop { if n > 2 { break; } drop(x); n += 1; }", "drop(x);"),
        ("for range", "for erg i in 0..3 { eat(packet: x); }", "x"),
        ("nested loop", "loop { loop { eat(packet: x); break; } }", "x"),
        ("nested for", "for erg i in 0..3 { for erg j in 0..3 { eat(packet: x); } }", "x"),
        (
            "struct field",
            "loop { if n > 2 { break; } erg h = Holder { held: x, }; drop(h); n += 1; }",
            "held: x",
        ),
        (
            "enum payload",
            "loop { if n > 2 { break; } erg s = Slot.Full(x); drop(s); n += 1; }",
            "x",
        ),
        (
            "erg binding",
            "loop { if n > 2 { break; } erg other = x; drop(other); n += 1; }",
            "erg other = x;",
        ),
        (
            "case arms",
            "loop { case n { 1 => { eat(packet: x); }, _ => { eat(packet: x); }, }; }",
            "x",
        ),
        (
            "continue after the move",
            "loop { n += 1; if n > 3 { break; } if n > 1 { eat(packet: x); continue; } }",
            "x",
        ),
    ] {
        let (source, result) = analyze_main(&format!("{SOURCE}{body}"));
        let error = result.expect_err(&format!("{label}: the move repeats on the next iteration"));
        assert!(
            matches!(&error.kind, SemanticErrorKind::MovedInPreviousIteration { name } if name == "x"),
            "{label}: {:?}",
            error.kind
        );
        assert_eq!(&source[error.span.start..error.span.end], moved_text, "{label}");
    }
}

#[test]
fn a_field_moved_in_a_loop_is_rejected_for_the_owner() {
    let (_, result) = analyze_main(
        "erg o = Owned { held: Buffer[4], }; for erg i in 0..3 { eat(packet: o.held); }",
    );
    let error = result.expect_err("the field is moved again by the next iteration");
    assert!(
        matches!(error.kind, SemanticErrorKind::MovedInPreviousIteration { name } if name == "o")
    );
}

#[test]
fn a_move_followed_by_break_or_return_is_accepted() {
    for (label, body) in [
        ("break", "loop { eat(packet: x); break; }"),
        ("if break", "loop { n += 1; if n > 2 { eat(packet: x); break; } }"),
        ("return", "loop { eat(packet: x); return; }"),
        ("nested block", "loop { { eat(packet: x); break; } }"),
        (
            "every case arm leaves",
            "loop { case n { 1 => { eat(packet: x); break; }, _ => { eat(packet: x); return; }, }; }",
        ),
        (
            "both branches leave",
            "loop { if n > 1 { eat(packet: x); break; } else { drop(x); return; } }",
        ),
    ] {
        let (_, result) = analyze_main(&format!("{SOURCE}{body}"));
        if let Err(error) = result {
            panic!("{label}: a move that leaves the loop is valid: {:?}", error.kind);
        }
    }
}

#[test]
fn a_value_declared_inside_the_loop_is_new_in_every_iteration() {
    for (label, body) in [
        (
            "loop",
            "erg n: Int = 0; loop { if n > 2 { break; } erg y: Buffer = Buffer[4]; eat(packet: y); n += 1; }",
        ),
        ("for range", "for erg i in 0..3 { erg y: Buffer = Buffer[4]; drop(y); }"),
        (
            "nested",
            "for erg i in 0..3 { erg y: Buffer = Buffer[4]; for erg j in 0..3 { append(y, 1u8); } drop(y); }",
        ),
    ] {
        let (_, result) = analyze_main(body);
        if let Err(error) = result {
            panic!("{label}: a value owned by the body is valid: {:?}", error.kind);
        }
    }
}

#[test]
fn a_loop_may_read_and_borrow_an_outer_value_without_moving_it() {
    let (_, result) = analyze_main(&format!(
        "{SOURCE}loop {{ if n > 2 {{ break; }} append(x, 1u8); n += 1; }} drop(x);"
    ));
    if let Err(error) = result {
        panic!("a read is not a move: {:?}", error.kind);
    }
}

#[test]
fn a_move_before_a_break_of_a_for_loop_is_rejected_at_the_loop() {
    // The normal exit of a `for` keeps the value, so its paths disagree.
    let (_, result) =
        analyze_main(&format!("{SOURCE}for erg i in 0..3 {{ eat(packet: x); break; }}"));
    let error = result.expect_err("the break path moves x and the normal exit keeps it");
    assert!(
        matches!(&error.kind, SemanticErrorKind::BranchStateMismatch { name, .. } if name == "x"),
        "{:?}",
        error.kind
    );
}

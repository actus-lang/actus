//! A call whose result can hold a view freezes every local passed to an `abs`
//! parameter of that call, exactly as building the holder directly would
//! (ADR-0046 2.3, bug 105). The callee may return a view of its `abs`
//! parameter, so the caller must keep the source alive and unchanged for as
//! long as the result lives, which is the end of the block that receives it.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Hv { abs b: Buffer, } struct Outer { erg inner: Hv, } \
struct Src { erg data: Buffer, } role Viewer { verb view_with(abs self: Src, abs extra: Buffer) -> Hv; } \
perform Viewer for Src { verb view_with(abs self: Src, abs extra: Buffer) -> Hv { return Hv { b: extra, }; } } \
verb make(abs src: Buffer) -> Hv { return Hv { b: src, }; } \
verb wrap(abs src: Buffer) -> Outer { return Outer { inner: Hv { b: src, }, }; } \
verb maybe(abs src: Buffer) -> Option[abs Buffer] { return Option[abs Buffer].Some(src); } \
verb measure(abs src: Buffer) -> Int { return buffer_length(buffer: abs src); } \
verb consume(dat held: Buffer) {} verb touch(ins held: Buffer) {} ";

fn analyze_main(body: &str) -> Result<(), SemanticErrorKind> {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map(|_| ()).map_err(|error| error.kind)
}

const LOCAL: &str = "erg local: Buffer = Buffer[4]; ";

#[test]
fn a_call_result_that_holds_a_view_freezes_the_abs_argument() {
    for (label, call, action, expected) in [
        ("drop", "make(src: abs local)", "drop(local);", "DropFrozen"),
        ("move to a binding", "make(src: abs local)", "erg other = local;", "MoveFrozen"),
        ("move to a dat call", "make(src: abs local)", "consume(held: local);", "MoveFrozen"),
        ("ins loan", "make(src: abs local)", "touch(held: ins local);", "InvalidArgumentRole"),
        ("nested holder", "wrap(src: abs local)", "drop(local);", "DropFrozen"),
        ("option payload", "maybe(src: abs local)", "drop(local);", "DropFrozen"),
    ] {
        let error = analyze_main(&format!("{LOCAL}erg held = {call}; {action}"))
            .expect_err(&format!("{label}: the source must stay frozen"));
        assert!(format!("{error:?}").starts_with(expected), "{label}: {error:?}");
    }
}

#[test]
fn a_method_result_that_holds_a_view_freezes_its_abs_arguments() {
    let error = analyze_main(&format!(
        "{LOCAL}erg source = Src {{ data: Buffer[4], }}; abs view = ref source; erg held = view.view_with(extra: abs local); erg other = local;"
    ))
    .expect_err("an abs argument of a view-holding method result stays frozen");
    assert!(format!("{error:?}").starts_with("MoveFrozen"), "{error:?}");
}

#[test]
fn a_chained_call_freezes_the_holder_that_the_view_is_read_from() {
    let error = analyze_main(&format!(
        "{LOCAL}erg first = make(src: abs local); erg second = make(src: abs first.b); erg other = first;"
    ))
    .expect_err("the second holder views the first, so the first stays frozen");
    assert!(format!("{error:?}").starts_with("MoveFrozen"), "{error:?}");
}

#[test]
fn a_call_result_view_ends_with_the_block_that_receives_it() {
    for (label, body) in [
        ("inner block", format!("{LOCAL}{{ erg held = make(src: abs local); }} drop(local);")),
        (
            "inner block then move",
            format!(
                "{LOCAL}{{ erg held = wrap(src: abs local); }} erg other = local; drop(other);"
            ),
        ),
        (
            "read while viewed",
            format!("{LOCAL}erg held = make(src: abs local); print(measure(src: abs local));"),
        ),
    ] {
        analyze_main(&body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn a_call_result_that_holds_no_view_leaves_the_argument_alone() {
    analyze_main(&format!("{LOCAL}erg size = measure(src: abs local); drop(local);"))
        .expect("an Int result cannot view its argument");
}

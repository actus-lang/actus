//! A struct, enum payload or `Option` that holds a view may leave a verb only
//! when every view in it comes from the verb's own `abs` parameters, as for a
//! direct `-> abs` return (ADR-0021, ADR-0046 2.3). A view of a local or of a
//! value that the verb owns would dangle once the verb returns.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticError, SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Hv { abs b: Buffer, } struct Wrap { erg inner: Hv, } \
struct Optional { abs b: Option[abs Buffer], } struct Holder { erg kept: Hv, } \
verb make(abs src: Buffer) -> Hv { return Hv { b: src, }; } ";

fn analyze_source(
    declarations: &str,
) -> (String, Result<actus::semantic::SemanticModel, SemanticError>) {
    let source = format!("{PRELUDE}{declarations} verb main() {{ }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    let result = analyze(&program);
    (source, result)
}

#[test]
fn a_returned_aggregate_cannot_hold_a_view_of_a_local() {
    for (label, declarations, culprit, returned) in [
        (
            "struct literal",
            "verb f() -> Hv { erg buf: Buffer = Buffer[4]; return Hv { b: buf, }; }",
            "buf",
            "Hv { b: buf, }",
        ),
        (
            "through a binding",
            "verb f() -> Hv { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; return h; }",
            "buf",
            "h",
        ),
        (
            "moved binding",
            "verb f() -> Hv { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; erg g = h; return g; }",
            "buf",
            "g",
        ),
        (
            "nested in an owned field",
            "verb f() -> Wrap { erg buf: Buffer = Buffer[4]; return Wrap { inner: Hv { b: buf, }, }; }",
            "buf",
            "Wrap { inner: Hv { b: buf, }, }",
        ),
        (
            "local abs alias",
            "verb f() -> Hv { erg buf: Buffer = Buffer[4]; abs v = ref buf; return Hv { b: v, }; }",
            "buf",
            "Hv { b: v, }",
        ),
        (
            "option payload",
            "verb f() -> Option[abs Buffer] { erg buf: Buffer = Buffer[4]; return Option[abs Buffer].Some(buf); }",
            "buf",
            "Option[abs Buffer].Some(buf)",
        ),
        (
            "option in an abs field",
            "verb f() -> Optional { erg buf: Buffer = Buffer[4]; return Optional { b: Option[abs Buffer].Some(buf), }; }",
            "buf",
            "Optional { b: Option[abs Buffer].Some(buf), }",
        ),
        (
            "result payload",
            "verb f() -> Result[Hv, Int] { erg buf: Buffer = Buffer[4]; return Result[Hv, Int].Ok(Hv { b: buf, }); }",
            "buf",
            "Result[Hv, Int].Ok(Hv { b: buf, })",
        ),
        (
            "owned parameter",
            "verb f(erg own: Buffer) -> Hv { return Hv { b: own, }; }",
            "own",
            "Hv { b: own, }",
        ),
        (
            "call result",
            "verb f() -> Hv { erg local: Buffer = Buffer[4]; return make(src: abs local); }",
            "local",
            "make(src: abs local)",
        ),
        (
            "temporary",
            "verb f() -> Hv { return Hv { b: Buffer[4], }; }",
            "temporary value",
            "Hv { b: Buffer[4], }",
        ),
        (
            "aggregate received as dat",
            "verb f(dat held: Hv) -> Hv { return held; }",
            "held",
            "held",
        ),
    ] {
        let (source, result) = analyze_source(declarations);
        let error = result.expect_err(&format!("{label}: the view would dangle"));
        assert!(
            matches!(&error.kind, SemanticErrorKind::BorrowedReturn { name } if name == culprit),
            "{label}: {:?}",
            error.kind
        );
        assert_eq!(&source[error.span.start..error.span.end], returned, "{label}");
    }
}

#[test]
fn an_ins_parameter_cannot_receive_a_view_of_a_local() {
    for (label, declarations) in [
        (
            "field",
            "verb f(ins out: Holder) { erg buf: Buffer = Buffer[4]; out.kept = Hv { b: buf, }; }",
        ),
        (
            "whole value",
            "verb f(ins out: Hv) { erg buf: Buffer = Buffer[4]; out = Hv { b: buf, }; }",
        ),
        (
            "through a binding",
            "verb f(ins out: Holder) { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; out.kept = h; }",
        ),
    ] {
        let (_, result) = analyze_source(declarations);
        let error = result.expect_err(&format!("{label}: the view would outlive the verb"));
        assert!(
            matches!(&error.kind, SemanticErrorKind::BorrowedReturn { name } if name == "buf"),
            "{label}: {:?}",
            error.kind
        );
    }
}

#[test]
fn an_aggregate_of_abs_parameter_views_may_be_returned_or_stored() {
    for (label, declarations) in [
        ("struct literal", "verb f(abs src: Buffer) -> Hv { return Hv { b: src, }; }"),
        (
            "through a binding",
            "verb f(abs src: Buffer) -> Hv { erg h = Hv { b: src, }; return h; }",
        ),
        (
            "moved binding",
            "verb f(abs src: Buffer) -> Hv { erg h = Hv { b: src, }; erg g = h; return g; }",
        ),
        ("nested", "verb f(abs src: Buffer) -> Wrap { return Wrap { inner: Hv { b: src, }, }; }"),
        (
            "result payload",
            "verb f(abs src: Buffer) -> Result[Hv, Int] { return Result[Hv, Int].Ok(Hv { b: src, }); }",
        ),
        ("call result", "verb f(abs src: Buffer) -> Hv { return make(src: abs src); }"),
        (
            "ins parameter field",
            "verb f(ins out: Holder, abs src: Buffer) { out.kept = Hv { b: src, }; }",
        ),
        (
            "plain data return",
            "verb f() -> Int { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; return buffer_length(buffer: abs h.b); }",
        ),
    ] {
        let (_, result) = analyze_source(declarations);
        if let Err(error) = result {
            panic!("{label}: every view comes from an abs parameter: {:?}", error.kind);
        }
    }
}

#[test]
fn a_local_aggregate_may_hold_a_view_and_be_passed_to_a_dat_parameter() {
    let (_, result) = analyze_source(
        "verb sink(dat held: Hv) { } verb f() { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; sink(held: dat h); }",
    );
    if let Err(error) = result {
        panic!("a view that stays inside the verb is valid: {:?}", error.kind);
    }
}

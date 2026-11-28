//! Values that no binding receives (ADR-0125, bug #128): a call result that a
//! statement discards and a call that a field or an element is read from are
//! released at the end of the expression that last reads them. A `?` nested in
//! an expression releases, when it fails, every value its statement already
//! evaluated.

use super::harness::{BACKENDS_BUILT, PRELUDE, run_on_every_backend};

/// `filled` returns a buffer whose first byte is 7, `pair` a `Pair` of the
/// trackers 1 and 2, `track` a tracker with the given id. `tracked` and
/// `count` fail with `Fail.Bad` unless `ok` holds.
const TEMPORARIES: &str = "enum Fail { Bad, } \
verb filled() -> Buffer { erg b: Buffer = Buffer[3]; b[0u32] = 7u8; return b; } \
verb pair() -> Pair { return Pair { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }; } \
verb track(erg id: Int) -> Tracker { return Tracker { id: id, }; } \
verb tracked(erg ok: Bool) -> Result[Tracker, Fail] { if ok { \
return Result[Tracker, Fail].Ok(Tracker { id: 8, }); } return Result[Tracker, Fail].Err(Fail.Bad); } \
verb count(erg ok: Bool) -> Result[Int, Fail] { if ok { return Result[Int, Fail].Ok(10); } \
return Result[Int, Fail].Err(Fail.Bad); } \
verb report(dat r: Result[Int, Fail]) { case dat r { Result.Ok(n) => { print(n); }, \
Result.Err(e) => { print(0); }, }; } ";

/// Runs `body` as the body of `main` after the declarations `verbs` and
/// checks its output and memory behaviour on every back end.
fn check_temporaries(label: &str, verbs: &str, body: &str, expected: &str) {
    let source = format!("{PRELUDE}{TEMPORARIES}{verbs}verb main() -> Int {{ {body} return 5; }}");
    assert_eq!(run_on_every_backend(label, &source, expected, 5), BACKENDS_BUILT);
}

#[test]
fn a_discarded_call_result_is_released_at_the_end_of_its_statement() {
    check_temporaries("discarded-buffer", "", "filled(); print(9);", "9\n");
    check_temporaries("discarded-drop", "", "pair(); print(9);", "2\n1\n9\n");
    check_temporaries(
        "discarded-in-branch",
        "",
        "erg n: Int = 1; if n > 0 { pair(); } print(9);",
        "2\n1\n9\n",
    );
}

#[test]
fn the_tail_of_a_returned_arm_written_with_a_semicolon_is_the_value_not_a_temporary() {
    let verbs = "verb choose(dat r: Result[Int, Fail]) -> Pair { return case dat r { \
Result.Ok(n) => { pair(); }, Result.Err(e) => { pair(); }, }; } ";
    check_temporaries(
        "arm-tail",
        verbs,
        "erg yes: Bool = true; erg r: Result[Int, Fail] = count(ok: yes); \
erg p: Pair = choose(r: dat r); print(9);",
        "9\n2\n1\n",
    );
}

#[test]
fn a_call_read_through_a_field_or_an_element_is_released_after_the_read() {
    check_temporaries("field-of-call", "", "erg n: Int = pair().a.id; print(n);", "2\n1\n1\n");
    check_temporaries(
        "element-of-call",
        "",
        "erg byte: u8 = filled()[0u32]; print(byte as Int);",
        "7\n",
    );
}

#[test]
fn a_failing_nested_try_releases_the_values_its_statement_evaluated() {
    let verbs = "verb run(erg ok: Bool) -> Result[Int, Fail] { erg one: Int = 1; \
erg n: Int = track(id: one).id + tracked(ok: ok)?.id; print(n); \
erg m: Int = count(ok: ok)? + count(ok: ok)?; return Result[Int, Fail].Ok(m); } ";
    check_temporaries(
        "nested-try",
        verbs,
        "erg yes: Bool = true; erg first: Result[Int, Fail] = run(ok: yes); report(r: dat first); \
erg no: Bool = false; erg second: Result[Int, Fail] = run(ok: no); report(r: dat second);",
        "1\n9\n8\n20\n1\n0\n",
    );
}

#[test]
fn a_failing_try_in_a_struct_literal_releases_the_fields_built_before_it() {
    let verbs = "verb build(erg ok: Bool) -> Result[Int, Fail] { erg three: Int = 3; \
erg p: Pair = Pair { a: track(id: three), b: tracked(ok: ok)?, }; print(6); \
return Result[Int, Fail].Ok(4); } ";
    check_temporaries(
        "nested-try-literal",
        verbs,
        "erg yes: Bool = true; erg first: Result[Int, Fail] = build(ok: yes); report(r: dat first); \
erg no: Bool = false; erg second: Result[Int, Fail] = build(ok: no); report(r: dat second);",
        "6\n8\n3\n4\n3\n0\n",
    );
}

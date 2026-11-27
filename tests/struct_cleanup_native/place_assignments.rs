//! An assignment to a field or an array element releases the value the place
//! held before it stores the new one (ADR-0125, bug #126): its own `Drop`, its
//! fields, a buffer's storage and an enum's active payload, through a nested
//! path, an `ins` loan and a run-time index alike. The assigned expression is
//! evaluated first, so it can still read the old value. A field that was moved
//! out holds nothing and becomes owned again.

use super::harness::{BACKENDS_BUILT, CFG, DEEP, PAIR, PRELUDE, run_on_every_backend};

/// Declarations the programs of this file use besides the shared prelude:
/// `Crate` owns a value with its own `Drop` contract, `Mail` an enum with a
/// buffer payload; `next` prints `100 + id` and returns `id + 10`; the
/// `ins` verbs replace a field or an element of a loan.
const PLACES: &str = "struct Crate { erg g: Guarded, } struct Mail { erg m: Msg, } \
verb next(abs t: Tracker) -> Int { print(100 + t.id); return t.id + 10; } \
verb sink_buffer(dat b: Buffer) {} \
verb refill(ins c: Cfg) { c.b = Buffer[4]; } \
verb retrack(ins p: Pair) { p.a = Tracker { id: 7, }; } \
verb restock(ins xs: Array[Tracker, 2]) { xs[1u32] = Tracker { id: 8, }; } ";

/// Runs `body` as the body of `main` with the declarations of this file and
/// checks its output and memory behaviour on every back end.
fn check_places(label: &str, body: &str, expected: &str) {
    let source = format!("{PRELUDE}{PLACES}verb main() -> Int {{ {body} return 5; }}");
    assert_eq!(run_on_every_backend(label, &source, expected, 5), BACKENDS_BUILT);
}

#[test]
fn assigning_a_buffer_field_releases_the_buffer_it_replaces() {
    check_places("field-buffer", &format!("{CFG} c.b = Buffer[4]; print(9);"), "9\n");
}

#[test]
fn assigning_a_field_runs_the_drop_of_the_value_it_replaces_first() {
    check_places(
        "field-drop",
        &format!("{PAIR} p.a = Tracker {{ id: 3, }}; print(9);"),
        "1\n9\n2\n3\n",
    );
}

#[test]
fn assigning_a_nested_field_releases_only_that_field() {
    check_places(
        "field-nested",
        &format!("{DEEP} d.p.b = Tracker {{ id: 4, }}; print(9);"),
        "2\n9\n3\n4\n1\n",
    );
}

#[test]
fn assigning_a_field_with_a_drop_contract_runs_it_and_releases_its_buffer() {
    check_places(
        "field-contract",
        "erg x: Crate = Crate { g: Guarded { buf: Buffer[1], id: 1, }, }; \
x.g = Guarded { buf: Buffer[2], id: 2, }; print(9);",
        "1\n9\n2\n",
    );
}

#[test]
fn assigning_an_enum_field_releases_the_payload_of_its_active_variant() {
    check_places(
        "field-enum",
        "erg m: Mail = Mail { m: Msg.Data(Buffer[2]), }; m.m = Msg.Empty; \
m.m = Msg.Data(Buffer[3]); print(9);",
        "9\n",
    );
}

#[test]
fn assigning_a_field_of_an_ins_loan_releases_the_value_the_caller_owned() {
    check_places("ins-buffer", &format!("{CFG} refill(c: ins c); print(9);"), "9\n");
    check_places("ins-drop", &format!("{PAIR} retrack(p: ins p); print(9);"), "1\n9\n2\n7\n");
}

#[test]
fn assigning_an_element_at_a_run_time_index_releases_the_element_it_replaces() {
    check_places(
        "element-drop",
        "erg xs: Array[Tracker, 2] = Array[Tracker, 2](); erg i: u32 = 1u32; \
xs[i] = Tracker { id: 3, }; xs[i] = Tracker { id: 4, }; print(9);",
        "0\n3\n9\n4\n0\n",
    );
    check_places(
        "element-buffer",
        "erg xs: Array[Buffer, 2] = Array[Buffer, 2](); erg i: u32 = 1u32; \
xs[i] = Buffer[2]; xs[i] = Buffer[3]; xs[0u32] = Buffer[4]; xs[0u32] = Buffer[5]; print(9);",
        "9\n",
    );
}

#[test]
fn assigning_an_element_of_an_ins_loan_or_of_a_field_releases_it() {
    check_places(
        "element-ins",
        "erg xs: Array[Tracker, 2] = Array[Tracker, 2](); restock(xs: ins xs); print(9);",
        "0\n9\n8\n0\n",
    );
    check_places(
        "element-field",
        "erg s: Shelf = Shelf { slots: Array[Buffer, 2](), }; erg i: u32 = 1u32; \
s.slots[i] = Buffer[2]; s.slots[i] = Buffer[3]; print(9);",
        "9\n",
    );
}

#[test]
fn assigning_a_field_of_an_element_releases_the_field_it_replaces() {
    check_places(
        "element-member",
        "erg hs: Array[Holder, 2] = Array[Holder, 2](); erg i: u32 = 0u32; \
hs[i] = Holder { buf: Buffer[2], }; hs[i].buf = Buffer[3]; print(9);",
        "9\n",
    );
}

#[test]
fn assigning_a_field_that_was_moved_out_releases_nothing_and_owns_it_again() {
    check_places(
        "moved-field",
        &format!("{PAIR} sink_tracker(t: dat p.a); p.a = Tracker {{ id: 3, }}; print(9);"),
        "1\n9\n2\n3\n",
    );
    check_places(
        "moved-buffer",
        &format!("{CFG} sink_buffer(b: dat c.b); c.b = Buffer[4]; print(9);"),
        "9\n",
    );
    check_places("moved-in-place", &format!("{CFG} c.b = grow(b: dat c.b); print(9);"), "9\n");
}

#[test]
fn assigning_a_field_with_a_moved_part_releases_the_rest_without_its_drop() {
    check_places(
        "moved-part",
        "erg x: Crate = Crate { g: Guarded { buf: Buffer[1], id: 1, }, }; \
sink_buffer(b: dat x.g.buf); x.g = Guarded { buf: Buffer[2], id: 2, }; print(9);",
        "9\n2\n",
    );
}

#[test]
fn the_assigned_value_is_evaluated_before_the_old_value_is_released() {
    check_places(
        "order-field",
        &format!("{PAIR} p.a = Tracker {{ id: next(t: abs p.a), }}; print(9);"),
        "101\n1\n9\n2\n11\n",
    );
    check_places(
        "order-owner",
        &format!(
            "{PAIR} p = Pair {{ a: Tracker {{ id: next(t: abs p.a), }}, b: Tracker {{ id: 3, }}, }}; \
print(9);"
        ),
        "101\n2\n1\n9\n3\n11\n",
    );
}

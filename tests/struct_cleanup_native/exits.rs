//! The exits of a scope: `return` (also one whose operand or `case` arm moves
//! the owner), `break`, `continue`, the end of an `if` branch, the value of a
//! branch, and an assignment that replaces an owner's value.

use super::harness::{
    BACKENDS_BUILT, CFG, GUARDED, HOLDER, PAIR, PRELUDE, check, run_on_every_backend,
};

#[test]
fn an_early_return_cleans_up_every_owned_aggregate_once() {
    let source = format!(
        "{PRELUDE}verb pick() -> Int {{ {CFG} {PAIR} {GUARDED} erg n: Int = 1; \
if n > 0 {{ return 5; }} print(8); return 6; }} \
verb main() -> Int {{ erg r: Int = pick(); print(9); return r; }}"
    );
    assert_eq!(run_on_every_backend("early-return", &source, "7\n2\n1\n9\n", 5), BACKENDS_BUILT);
}

#[test]
fn a_break_cleans_up_the_aggregates_of_the_loop_body_once() {
    check("break", &format!("loop {{ {CFG} {PAIR} print(8); break; }} print(9);"), "8\n2\n1\n9\n");
}

#[test]
fn a_dat_parameter_consumed_on_one_return_path_is_cleaned_up_once_on_each_path() {
    for (label, condition, expected) in
        [("param-taken", 1, "2\n1\n9\n"), ("param-fallthrough", 0, "8\n2\n1\n9\n")]
    {
        let source = format!(
            "{PRELUDE}verb route(dat p: Pair) {{ erg n: Int = {condition}; \
if n > 0 {{ drop_pair(p: dat p); return; }} print(8); }} \
verb main() -> Int {{ {PAIR} route(p: dat p); print(9); return 5; }}"
        );
        assert_eq!(run_on_every_backend(label, &source, expected, 5), BACKENDS_BUILT);
    }
}

#[test]
fn a_local_consumed_on_one_return_path_is_cleaned_up_once_on_each_path() {
    for (label, condition, expected) in
        [("local-taken", 1, "2\n1\n9\n"), ("local-fallthrough", 0, "8\n2\n1\n9\n")]
    {
        let source = format!(
            "{PRELUDE}verb route() {{ {PAIR} erg n: Int = {condition}; \
if n > 0 {{ drop_pair(p: dat p); return; }} print(8); }} \
verb main() -> Int {{ route(); print(9); return 5; }}"
        );
        assert_eq!(run_on_every_backend(label, &source, expected, 5), BACKENDS_BUILT);
    }
}

#[test]
fn a_loop_that_returns_cleans_up_the_owners_of_every_scope_it_leaves() {
    let source = format!(
        "{PRELUDE}verb search() -> Int {{ {CFG} erg n: Int = 0; \
loop {{ {PAIR} n += 1; if n > 1 {{ return n; }} print(8); }} return 0; }} \
verb main() -> Int {{ erg r: Int = search(); print(9); return 5; }}"
    );
    assert_eq!(
        run_on_every_backend("loop-return", &source, "8\n2\n1\n2\n1\n9\n", 5),
        BACKENDS_BUILT
    );
}

#[test]
fn a_return_whose_operand_moves_the_owner_does_not_release_it_again() {
    let source = format!(
        "{PRELUDE}verb relay() -> Int {{ {PAIR} return count_pair(p: dat p); }} \
verb main() -> Int {{ erg r: Int = relay(); print(r); return 5; }}"
    );
    assert_eq!(run_on_every_backend("return-moves", &source, "2\n1\n5\n", 5), BACKENDS_BUILT);
}

#[test]
fn a_returned_case_releases_an_owner_only_where_no_arm_moved_it() {
    for (label, flag, expected) in
        [("case-arm-moves", "true", "2\n1\n5\n"), ("case-arm-diverges", "false", "2\n1\n7\n")]
    {
        let source = format!(
            "{PRELUDE}verb route(erg flag: Bool) -> Int {{ {PAIR} \
return case flag {{ true => count_pair(p: dat p), _ => {{ return 7; }}, }}; }} \
verb main() -> Int {{ erg f: Bool = {flag}; erg r: Int = route(flag: erg f); print(r); return 5; }}"
        );
        assert_eq!(run_on_every_backend(label, &source, expected, 5), BACKENDS_BUILT);
    }
}

#[test]
fn a_continue_releases_the_owners_of_the_iteration_it_ends() {
    check(
        "continue",
        &format!(
            "erg i: Int = 0; loop {{ i += 1; if i > 2 {{ break; }} {CFG} \
erg t: Tracker = Tracker {{ id: i, }}; if i == 1 {{ continue; }} print(8); }} print(9);"
        ),
        "1\n8\n2\n9\n",
    );
}

#[test]
fn an_owner_declared_in_an_if_branch_is_released_where_the_branch_ends() {
    check(
        "branch-owner",
        "erg n: Int = 1; if n > 0 { erg t: Tracker = Tracker { id: 3, }; erg b: Buffer = Buffer[1]; \
print(8); } else { erg t: Tracker = Tracker { id: 4, }; } print(9);",
        "8\n3\n9\n",
    );
}

#[test]
fn the_value_of_an_if_branch_moves_out_of_the_branch() {
    check(
        "branch-value",
        "erg n: Int = 1; erg kept: Buffer = if n > 0 { erg b: Buffer = Buffer[1]; b } \
else { Buffer[2] }; erg other: Buffer = if n > 1 { Buffer[3] } else { erg c: Buffer = Buffer[4]; \
c }; print(9);",
        "9\n",
    );
}

#[test]
fn an_assignment_releases_the_value_it_replaces() {
    check(
        "replace",
        "erg t: Tracker = Tracker { id: 1, }; t = Tracker { id: 2, }; erg b: Buffer = Buffer[1]; \
b = Buffer[2]; print(9);",
        "1\n9\n2\n",
    );
}

#[test]
fn an_owner_given_back_its_own_moved_value_is_live_again() {
    check(
        "regrow",
        "erg b: Buffer = Buffer[1]; b = grow(b: dat b); b = grow(b: dat b); print(9);",
        "9\n",
    );
}

#[test]
fn a_buffer_reassigned_in_a_loop_releases_each_value_once() {
    check(
        "loop-carried",
        "erg b: Buffer = Buffer[1]; erg i: Int = 0; loop { if i > 2 { break; } b = Buffer[2]; \
i += 1; } print(9);",
        "9\n",
    );
}

#[test]
fn a_returned_field_leaves_its_owner_and_the_rest_is_released_once() {
    let source = format!(
        "{PRELUDE}verb take_a(dat p: Pair) -> Tracker {{ return p.a; }} \
verb take_buf(dat h: Holder) -> Buffer {{ return h.buf; }} \
verb main() -> Int {{ {PAIR} {HOLDER} erg t: Tracker = take_a(p: dat p); \
erg b: Buffer = take_buf(h: dat h); print(9); return 5; }}"
    );
    assert_eq!(run_on_every_backend("returned-field", &source, "2\n9\n1\n", 5), BACKENDS_BUILT);
}

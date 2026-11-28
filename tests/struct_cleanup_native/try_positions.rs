//! Bug #130: a `?` in the value of an `if` or `case` arm, in the right operand
//! of `&&` or `||`, in an `if` or `else if` condition, in a `case` subject or in
//! a compound assignment returns early with its error and releases exactly
//! the owners that are live there (ADR-0125), on every back end. Evaluation
//! order and short circuits are kept: an arm that is not taken and the right
//! operand that `&&` or `||` skips are not evaluated, so their `?` neither
//! runs nor returns.

use super::harness::{BACKENDS_BUILT, PRELUDE, run_on_every_backend};

/// `count` prints 7 when it is called and gives `Ok(10)` or `Err(Fail.Bad)`;
/// `flag` prints 7 and gives `Ok(true)` or the error. `report` prints the
/// value of an `Ok` and 0 for an `Err`.
const VERBS: &str = "enum Fail { Bad, } \
verb count(erg ok: Bool) -> Result[Int, Fail] { print(7); if ok { return Result[Int, Fail].Ok(10); } \
return Result[Int, Fail].Err(Fail.Bad); } \
verb flag(erg ok: Bool) -> Result[Bool, Fail] { print(7); if ok { return Result[Bool, Fail].Ok(true); } \
return Result[Bool, Fail].Err(Fail.Bad); } \
verb report(dat r: Result[Int, Fail]) { case dat r { Result.Ok(n) => { print(n); }, \
Result.Err(e) => { print(0); }, }; } ";

/// Runs `body` as the body of `run(ok)`, which owns the tracker 1, once with
/// `ok` true and once with it false, reports both results and checks the
/// output and the memory behaviour on every back end.
fn check_try(label: &str, body: &str, expected: &str) {
    let source = format!(
        "{PRELUDE}{VERBS}verb run(erg ok: Bool) -> Result[Int, Fail] {{ \
erg t: Tracker = Tracker {{ id: 1, }}; {body} }} \
verb main() -> Int {{ erg yes: Bool = true; erg a: Result[Int, Fail] = run(ok: yes); \
report(r: dat a); erg no: Bool = false; erg b: Result[Int, Fail] = run(ok: no); \
report(r: dat b); return 5; }}"
    );
    assert_eq!(run_on_every_backend(label, &source, expected, 5), BACKENDS_BUILT);
}

const OK_X: &str = "print(x); return Result[Int, Fail].Ok(x);";

#[test]
fn a_try_in_the_value_of_an_if_arm_returns_early_from_the_verb() {
    let body = format!(
        "erg pick: Bool = true; erg x: Int = if pick {{ count(ok: ok)? }} else {{ 3 }}; {OK_X}"
    );
    check_try("if-arm", &body, "7\n10\n1\n10\n7\n1\n0\n");
    let skipped = format!(
        "erg pick: Bool = false; erg x: Int = if pick {{ count(ok: ok)? }} else {{ 3 }}; {OK_X}"
    );
    check_try("if-arm-not-taken", &skipped, "3\n1\n3\n3\n1\n3\n");
}

#[test]
fn a_try_statement_in_an_arm_releases_the_owners_of_the_arm() {
    let body = format!(
        "erg pick: Bool = true; erg x: Int = if pick {{ erg u: Tracker = Tracker {{ id: 2, }}; \
erg y: Int = count(ok: ok)?; y + 1 }} else {{ 3 }}; {OK_X}"
    );
    check_try("arm-statement", &body, "7\n2\n11\n1\n11\n7\n2\n1\n0\n");
}

#[test]
fn a_try_in_the_value_of_a_case_arm_returns_early_from_the_verb() {
    let body =
        format!("erg k: Int = 1; erg x: Int = case k {{ 1 => count(ok: ok)?, _ => 3, }}; {OK_X}");
    check_try("case-arm", &body, "7\n10\n1\n10\n7\n1\n0\n");
    check_try(
        "returned-case-arm",
        "erg k: Int = 1; return case k { 1 => Result[Int, Fail].Ok(count(ok: ok)?), \
_ => Result[Int, Fail].Ok(3), };",
        "7\n1\n10\n7\n1\n0\n",
    );
}

#[test]
fn a_try_on_the_right_of_and_or_or_runs_only_when_it_is_evaluated() {
    let tail = "if b { return Result[Int, Fail].Ok(1); } return Result[Int, Fail].Ok(2);";
    let and = |left: &str| {
        format!("erg left: Bool = {left}; erg b: Bool = left && flag(ok: ok)?; {tail}")
    };
    let or = |left: &str| {
        format!("erg left: Bool = {left}; erg b: Bool = left || flag(ok: ok)?; {tail}")
    };
    check_try("and-right", &and("true"), "7\n1\n1\n7\n1\n0\n");
    check_try("and-short-circuit", &and("false"), "1\n2\n1\n2\n");
    check_try("or-right", &or("false"), "7\n1\n1\n7\n1\n0\n");
    check_try("or-short-circuit", &or("true"), "1\n1\n1\n1\n");
}

#[test]
fn a_try_in_a_condition_returns_before_either_branch_runs() {
    let ok4 = "return Result[Int, Fail].Ok(4);";
    check_try(
        "if-condition",
        &format!("if flag(ok: ok)? {{ print(3); }} {ok4}"),
        "7\n3\n1\n4\n7\n1\n0\n",
    );
    let else_if = |first: &str| {
        format!(
            "erg a: Bool = {first}; if a {{ print(1); }} else if flag(ok: ok)? {{ print(3); }} {ok4}"
        )
    };
    check_try("else-if-condition", &else_if("false"), "7\n3\n1\n4\n7\n1\n0\n");
    check_try("else-if-not-reached", &else_if("true"), "1\n1\n4\n1\n1\n4\n");
}

#[test]
fn a_try_in_a_case_subject_or_a_compound_assignment_returns_early() {
    check_try(
        "case-subject",
        "erg x: Int = case count(ok: ok)? { 10 => 1, _ => 2, }; return Result[Int, Fail].Ok(x);",
        "7\n1\n1\n7\n1\n0\n",
    );
    check_try(
        "compound-assignment",
        "erg x: Int = 0; x += count(ok: ok)?; return Result[Int, Fail].Ok(x);",
        "7\n1\n10\n7\n1\n0\n",
    );
}

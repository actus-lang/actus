//! A view held by a value belongs to the place that holds the value
//! (ADR-0046 2.3, bug 106). Every program in `check_rejects_*` stored a view
//! into a place that outlives its source, or let the source go while the view
//! was still reachable, and used to be accepted. The programs in
//! `check_accepts_*` keep the source at least as long as the holder.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-view-lifetimes-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Hv { abs b: Buffer, }\nstruct Holder { erg kept: Hv, }\n";

fn assert_rejected(label: &str, body: &str, code: &str, name: &str) {
    let output = actus(label, &format!("{PRELUDE}{body}"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{label}: must be rejected");
    assert!(stderr.contains(code), "{label}: expected {code}: {stderr}");
    assert!(name.is_empty() || stderr.contains(&format!("`{name}`")), "{label}: {stderr}");
}

fn assert_accepted(label: &str, body: &str) {
    let output = actus(label, &format!("{PRELUDE}{body}"));
    assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn check_rejects_dropping_the_source_of_an_if_value() {
    assert_rejected(
        "if-drop",
        "verb f() -> Int {
    erg c: Bool = true;
    erg local: Buffer = Buffer[4];
    erg other: Buffer = Buffer[8];
    erg h = if c { Hv { b: local, } } else { Hv { b: other, } };
    drop(local);
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
        "E1009",
        "local",
    );
}

#[test]
fn check_rejects_returning_an_if_or_case_value_that_views_a_local() {
    assert_rejected(
        "if-return",
        "verb f(abs c: Bool) -> Hv {
    erg local: Buffer = Buffer[4];
    erg other: Buffer = Buffer[8];
    return if c { Hv { b: local, } } else { Hv { b: other, } };
}
verb main() -> Int { return 0; }",
        "E1018",
        "local",
    );
    assert_rejected(
        "case-return",
        "verb f(abs n: Int) -> Hv {
    erg local: Buffer = Buffer[4];
    return case n { 0 => Hv { b: local, }, _ => Hv { b: local, }, };
}
verb main() -> Int { return 0; }",
        "E1018",
        "local",
    );
}

#[test]
fn check_rejects_returning_a_view_bound_by_a_case_pattern() {
    assert_rejected(
        "pattern-return",
        "verb f(abs fallback: Buffer) -> Hv {
    erg local: Buffer = Buffer[4];
    erg opt = Option[abs Buffer].Some(local);
    case opt {
        Option.Some(inner) => { return Hv { b: inner, }; },
        Option.None => { },
    };
    return Hv { b: fallback, };
}
verb main() -> Int { return 0; }",
        "E1018",
        "local",
    );
    assert_rejected(
        "pattern-dat",
        "verb f(abs fallback: Buffer) -> Hv {
    erg local: Buffer = Buffer[4];
    erg w = Option[Hv].Some(Hv { b: local, });
    case dat w {
        Option.Some(hv) => { return hv; },
        Option.None => { },
    };
    return Hv { b: fallback, };
}
verb main() -> Int { return 0; }",
        "E1018",
        "local",
    );
}

#[test]
fn check_rejects_storing_a_view_of_an_inner_local_in_an_outer_binding() {
    assert_rejected(
        "outer-local",
        "verb f() -> Int {
    erg keep: Buffer = Buffer[1];
    erg h = Hv { b: keep, };
    {
        erg local: Buffer = Buffer[4];
        h = Hv { b: local, };
    }
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
        "E1018",
        "local",
    );
    assert_rejected(
        "decl-from-if",
        "verb f() -> Int {
    erg c: Bool = true;
    erg keep: Buffer = Buffer[1];
    erg h = Hv { b: keep, };
    if c {
        erg local: Buffer = Buffer[4];
        h = Hv { b: local, };
    }
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
        "E1018",
        "local",
    );
}

#[test]
fn check_rejects_storing_a_view_of_an_inner_local_in_an_outer_field_or_element() {
    assert_rejected(
        "outer-field",
        "verb f() -> Int {
    erg keep: Buffer = Buffer[1];
    erg outer = Holder { kept: Hv { b: keep, }, };
    {
        erg local: Buffer = Buffer[4];
        outer.kept = Hv { b: local, };
    }
    return buffer_length(buffer: abs outer.kept.b);
}
verb main() -> Int { return f(); }",
        "E1018",
        "local",
    );
    assert_rejected(
        "local-array",
        "verb f() -> Int {
    erg keep: Buffer = Buffer[1];
    erg xs: Array[Hv, 2] = Array[Hv, 2]();
    {
        erg local: Buffer = Buffer[4];
        xs[0] = Hv { b: local, };
    }
    return buffer_length(buffer: abs xs[0].b);
}
verb main() -> Int { return f(); }",
        "E1018",
        "local",
    );
}

#[test]
fn check_rejects_storing_a_view_of_a_local_through_an_ins_element() {
    assert_rejected(
        "ins-array",
        "verb f(ins out: Array[Hv, 2]) {
    erg local: Buffer = Buffer[4];
    out[0] = Hv { b: local, };
}
verb main() -> Int { return 0; }",
        "E1018",
        "local",
    );
}

#[test]
fn check_freezes_the_source_of_a_view_stored_in_an_element() {
    assert_rejected(
        "element-freeze",
        "verb f() -> Int {
    erg xs: Array[Hv, 2] = Array[Hv, 2]();
    erg local: Buffer = Buffer[4];
    xs[0] = Hv { b: local, };
    drop(local);
    return buffer_length(buffer: abs xs[0].b);
}
verb main() -> Int { return f(); }",
        "E1009",
        "local",
    );
}

#[test]
fn check_accepts_views_whose_sources_live_as_long_as_the_holder() {
    assert_accepted(
        "if-same-scope",
        "verb f() -> Int {
    erg c: Bool = true;
    erg local: Buffer = Buffer[4];
    erg other: Buffer = Buffer[8];
    erg h = if c { Hv { b: local, } } else { Hv { b: other, } };
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
    );
    assert_accepted(
        "if-inner-holder-then-drop",
        "verb f() -> Int {
    erg c: Bool = true;
    erg local: Buffer = Buffer[4];
    erg other: Buffer = Buffer[8];
    erg size: Int = 0;
    {
        erg h = if c { Hv { b: local, } } else { Hv { b: other, } };
        size = buffer_length(buffer: abs h.b);
    }
    drop(local);
    drop(other);
    return size;
}
verb main() -> Int { return f(); }",
    );
    assert_accepted(
        "assign-in-branch",
        "verb f() -> Int {
    erg c: Bool = true;
    erg keep: Buffer = Buffer[1];
    erg keep2: Buffer = Buffer[2];
    erg h = Hv { b: keep, };
    if c {
        h = Hv { b: keep2, };
    }
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
    );
    assert_accepted(
        "assign-same-scope-source",
        "verb f() -> Int {
    erg keep: Buffer = Buffer[1];
    erg h = Hv { b: keep, };
    {
        h = Hv { b: keep, };
    }
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
    );
}

#[test]
fn check_keeps_the_source_frozen_after_a_branch_that_stored_its_view() {
    assert_rejected(
        "frozen-after-branch",
        "verb f() -> Int {
    erg c: Bool = true;
    erg keep: Buffer = Buffer[1];
    erg keep2: Buffer = Buffer[2];
    erg h = Hv { b: keep, };
    if c {
        h = Hv { b: keep2, };
    }
    drop(keep2);
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
        "E1009",
        "keep2",
    );
}

#[test]
fn check_releases_a_view_stored_in_a_branch_that_returns() {
    assert_accepted(
        "branch-returns",
        "verb f() -> Int {
    erg c: Bool = true;
    erg keep: Buffer = Buffer[1];
    erg keep2: Buffer = Buffer[2];
    erg h = Hv { b: keep, };
    if c {
        h = Hv { b: keep2, };
        return buffer_length(buffer: abs h.b);
    }
    drop(keep2);
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
    );
}

#[test]
fn check_keeps_a_view_stored_in_a_branch_that_does_not_return_frozen() {
    assert_rejected(
        "branch-continues",
        "verb f() -> Int {
    erg c: Bool = true;
    erg keep: Buffer = Buffer[1];
    erg keep2: Buffer = Buffer[2];
    erg h = Hv { b: keep, };
    if c {
        h = Hv { b: keep2, };
        print(1);
    }
    drop(keep2);
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
        "E1009",
        "keep2",
    );
}

#[test]
fn check_releases_a_view_stored_in_a_case_arm_that_returns() {
    assert_accepted(
        "arm-returns",
        "verb f(abs n: Int) -> Int {
    erg keep: Buffer = Buffer[1];
    erg keep2: Buffer = Buffer[2];
    erg h = Hv { b: keep, };
    case n {
        0 => { h = Hv { b: keep2, }; return buffer_length(buffer: abs h.b); },
        _ => { },
    };
    drop(keep2);
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { erg n: Int = 1; return f(n: abs n); }",
    );
}

#[test]
fn check_keeps_a_view_stored_in_a_case_arm_that_joins_frozen() {
    assert_rejected(
        "arm-joins",
        "verb f(abs n: Int) -> Int {
    erg keep: Buffer = Buffer[1];
    erg keep2: Buffer = Buffer[2];
    erg h = Hv { b: keep, };
    case n {
        0 => { h = Hv { b: keep2, }; },
        _ => { },
    };
    drop(keep2);
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { erg n: Int = 1; return f(n: abs n); }",
        "E1009",
        "keep2",
    );
}

#[test]
fn check_accepts_a_view_of_an_abs_parameter_bound_by_a_case_pattern() {
    assert_accepted(
        "pattern-abs-parameter",
        "verb g(abs o: Option[abs Buffer], abs fallback: Buffer) -> Hv {
    case o {
        Option.Some(inner) => { return Hv { b: inner, }; },
        Option.None => { },
    };
    return Hv { b: fallback, };
}
verb main() -> Int { return 0; }",
    );
}

#[test]
fn check_accepts_a_case_dat_view_kept_in_the_verb() {
    assert_accepted(
        "pattern-dat-local",
        "verb f() -> Int {
    erg local: Buffer = Buffer[4];
    erg w = Option[Hv].Some(Hv { b: local, });
    erg size: Int = 0;
    case dat w {
        Option.Some(hv) => { size = buffer_length(buffer: abs hv.b); },
        Option.None => { },
    };
    return size;
}
verb main() -> Int { return f(); }",
    );
    assert_accepted(
        "pattern-dat-expression",
        "verb f() -> Int {
    erg local: Buffer = Buffer[4];
    erg w = Option[Hv].Some(Hv { b: local, });
    erg h = case dat w { Option.Some(hv) => hv, Option.None => { return 0; }, };
    return buffer_length(buffer: abs h.b);
}
verb main() -> Int { return f(); }",
    );
}

//! Moving one field of an owner leaves its other fields usable (ADR-0016,
//! ADR-0046 2.2). These tests drive the CLI so that the diagnostic codes and
//! the exit status are part of the contract, and run the cleanup of a partially
//! moved owner on every back end that is built in: the fields still owned are
//! released exactly once, the moved ones never, observed by the output of a
//! `Drop` role and by valgrind when it is installed.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

#[cfg(feature = "llvm")]
const LLVM: Option<&str> = Some("llvm");
#[cfg(not(feature = "llvm"))]
const LLVM: Option<&str> = None;
#[cfg(feature = "cranelift")]
const CRANELIFT: Option<&str> = Some("cranelift");
#[cfg(not(feature = "cranelift"))]
const CRANELIFT: Option<&str> = None;

fn actus(label: &str, command: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-partial-move-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([command, input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

/// Builds `source` on every back end and returns each run's stdout, status
/// code and, when `valgrind` is true and installed, the valgrind exit code.
fn run_on_every_backend(label: &str, source: &str, valgrind: bool) -> Vec<(String, String, i32)> {
    let mut runs = Vec::new();
    for backend in [LLVM, CRANELIFT].into_iter().flatten() {
        let root = std::env::temp_dir()
            .join(format!("actus-partial-move-{label}-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let binary = root.with_extension("bin");
        fs::write(&input, source).expect("write fixture");
        let build = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["build", input.to_str().unwrap(), "--backend", backend, "--emit", "exe", "-o"])
            .arg(&binary)
            .output()
            .expect("start the build");
        assert!(
            build.status.success(),
            "{label} on {backend}: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let output = if valgrind {
            Command::new("valgrind")
                .args(["--error-exitcode=99", "--leak-check=full", "--errors-for-leak-kinds=all"])
                .arg("--show-leak-kinds=all")
                .arg(&binary)
                .output()
                .expect("start valgrind")
        } else {
            Command::new(&binary).output().expect("run the program")
        };
        runs.push((
            backend.to_owned(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            output.status.code().unwrap_or(-1),
        ));
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(binary);
    }
    runs
}

fn valgrind_installed() -> bool {
    Command::new("valgrind").arg("--version").output().is_ok_and(|output| output.status.success())
}

const PRELUDE: &str = "struct Cfg { erg a: Buffer, erg b: Buffer, } \
struct Inner { erg a: Buffer, erg b: Buffer, } struct Outer { erg inner: Inner, erg z: Buffer, } \
verb sink(dat x: Buffer) { drop(x); } verb take(dat x: Cfg) { drop(x); } \
verb take_inner(dat x: Inner) { drop(x); } verb length(abs x: Buffer) -> Int { return 1; } ";
const CFG: &str = "erg c: Cfg = Cfg { a: Buffer[1], b: Buffer[1], }; ";
const OUTER: &str =
    "erg c: Outer = Outer { inner: Inner { a: Buffer[1], b: Buffer[1], }, z: Buffer[1], }; ";

#[test]
fn check_accepts_moving_and_reading_disjoint_fields() {
    for (label, body) in [
        ("two-fields", format!("verb main() {{ {CFG} sink(x: dat c.a); sink(x: dat c.b); }}")),
        (
            "read-other-field",
            format!("verb main() {{ {CFG} sink(x: dat c.a); erg n: Int = length(x: abs c.b); }}"),
        ),
        ("dat-parameter", "verb f(dat c: Cfg) { sink(x: dat c.a); sink(x: dat c.b); }".to_owned()),
        (
            "nested",
            format!(
                "verb main() {{ {OUTER} sink(x: dat c.inner.a); sink(x: dat c.inner.b); sink(x: dat c.z); }}"
            ),
        ),
        ("drop-rest", format!("verb main() {{ {CFG} sink(x: dat c.a); drop(c); }}")),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}{body}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn check_rejects_whole_use_and_repeated_moves_after_a_partial_move() {
    for (label, body, code) in [
        (
            "same-field",
            format!("verb main() {{ {CFG} sink(x: dat c.a); sink(x: dat c.a); }}"),
            "E1005",
        ),
        (
            "whole-move",
            format!("verb main() {{ {CFG} sink(x: dat c.a); take(x: dat c); }}"),
            "E1005",
        ),
        (
            "parent-after-child",
            format!(
                "verb main() {{ {OUTER} sink(x: dat c.inner.a); take_inner(x: dat c.inner); }}"
            ),
            "E1005",
        ),
        (
            "child-after-parent",
            format!(
                "verb main() {{ {OUTER} take_inner(x: dat c.inner); sink(x: dat c.inner.a); }}"
            ),
            "E1005",
        ),
        (
            "second-drop",
            format!("verb main() {{ {CFG} sink(x: dat c.a); drop(c); drop(c); }}"),
            "E1007",
        ),
        (
            "whole-move-after-drop",
            format!("verb main() {{ {CFG} sink(x: dat c.a); drop(c); take(x: dat c); }}"),
            "E1006",
        ),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}{body}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains(code), "{label}: {stderr}");
    }
}

const TRACKED: &str = "struct Tracker { id: Int, } \
perform Drop for Tracker { verb drop(ins self: Tracker) { print(self.id); } } \
struct Pair { erg a: Tracker, erg b: Tracker, } verb consume(dat x: Tracker) { drop(x); } ";

#[test]
fn native_cleanup_of_a_partially_moved_owner_releases_the_rest_once() {
    for (label, body, expected) in [
        // `a` is released by the callee, the explicit drop releases only `b`.
        (
            "explicit-drop",
            "erg c = Pair { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }; consume(x: dat c.a); print(7); drop(c); print(8);",
            "1\n7\n2\n8\n",
        ),
        // The scope end releases only the field that stayed.
        (
            "scope-end",
            "erg c = Pair { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }; consume(x: dat c.b); print(7);",
            "2\n7\n1\n",
        ),
        // Every field moved: nothing is left for the owner to release.
        (
            "all-moved",
            "erg c = Pair { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }; consume(x: dat c.a); consume(x: dat c.b); print(7);",
            "1\n2\n7\n",
        ),
    ] {
        let source = format!("{TRACKED}verb main() -> Int {{ {body} return 42; }}");
        for (backend, stdout, code) in run_on_every_backend(label, &source, false) {
            assert_eq!(code, 42, "{label} on {backend}");
            assert_eq!(stdout, expected, "{label} on {backend}");
        }
    }
}

#[test]
fn native_cleanup_of_a_partially_moved_owner_has_no_double_free_and_no_leak() {
    if !valgrind_installed() {
        eprintln!("NOT RUN: valgrind is not installed");
        return;
    }
    for (label, body) in [
        ("explicit-drop", "sink(x: dat c.a); drop(c);"),
        ("scope-end", "sink(x: dat c.b);"),
        ("all-moved", "sink(x: dat c.a); sink(x: dat c.b);"),
    ] {
        let source = format!("{PRELUDE}verb main() -> Int {{ {CFG} {body} return 42; }}");
        for (backend, _, code) in run_on_every_backend(label, &source, true) {
            assert_eq!(code, 42, "{label} on {backend}: valgrind reported an error or a leak");
        }
    }
}

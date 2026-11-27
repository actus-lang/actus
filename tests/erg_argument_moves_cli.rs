//! An `erg` parameter takes ownership of a move-only argument (ADR-0016,
//! ADR-0059): the callee cleans it up, so the caller cannot use it again. A
//! program that read the argument afterwards used to be accepted and freed the
//! same buffer twice. These tests drive the CLI so that the diagnostic code and
//! the exit status are part of the contract, and run the accepted forms.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, command: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-erg-argument-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([command, input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Node { value: Int, } verb hold_buffer(erg held: Buffer) {} \
verb hold_arena(erg held: Arena[64]) {} verb hold_count(erg held: Int) {} ";

#[test]
fn check_rejects_use_of_a_source_that_an_erg_parameter_took() {
    for (label, body, code) in [
        (
            "buffer-append",
            "erg buf: Buffer = Buffer[4]; hold_buffer(held: buf); append(buf, 1u8);",
            "E1005",
        ),
        ("buffer-drop", "erg buf: Buffer = Buffer[4]; hold_buffer(held: buf); drop(buf);", "E1007"),
        (
            "buffer-explicit-role",
            "erg buf: Buffer = Buffer[4]; hold_buffer(held: erg buf); drop(buf);",
            "E1007",
        ),
        (
            "arena-place",
            "erg arena: Arena[64] = Arena[64](); hold_arena(held: arena); erg node = arena.place(value: Node { value: 1, });",
            "E1005",
        ),
        (
            "twice",
            "erg buf: Buffer = Buffer[4]; hold_buffer(held: buf); hold_buffer(held: buf);",
            "E1005",
        ),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains(code), "{label}: {stderr}");
    }
}

#[test]
fn check_keeps_rejecting_a_view_given_to_an_erg_parameter() {
    let body = "erg buf: Buffer = Buffer[4]; abs view = ref buf; hold_buffer(held: buf);";
    let output = actus("frozen", "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E1016"));
}

#[test]
fn check_accepts_scalars_and_owners_nobody_reads_again() {
    for (label, body) in [
        (
            "scalar",
            "erg count: Int = 1; hold_count(held: count); hold_count(held: count); print(count);",
        ),
        ("buffer", "erg buf: Buffer = Buffer[4]; hold_buffer(held: buf); print(1);"),
        ("arena", "erg arena: Arena[64] = Arena[64](); hold_arena(held: arena); print(1);"),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn executes_a_buffer_given_to_an_erg_parameter_without_a_double_free() {
    let source = "verb hold(erg held: Buffer) -> Int { return held[4u32] as Int; } \
verb main() -> Int { erg buf: Buffer = Buffer[4]; append(buf, 42u8); return hold(held: buf); }";
    let root = std::env::temp_dir().join(format!("actus-erg-argument-run-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, source).expect("write fixture");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().unwrap(), "--emit", "exe", "-o", output.to_str().unwrap()])
        .output()
        .expect("start build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let status = Command::new(&output).status().expect("run fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

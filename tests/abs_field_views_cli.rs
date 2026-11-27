//! An `abs` struct field is a read-only view that freezes its source while the
//! holder lives (ADR-0046 2.3). A program that dropped or moved the source
//! while the holder still pointed at it used to be accepted. These tests drive
//! the CLI so that the diagnostic code and the exit status are part of the
//! contract, and run the accepted form.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, command: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-abs-field-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([command, input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Plain { number: Int, } struct ViewBuffer { abs held: Buffer, } \
struct ViewPlain { abs held: Plain, } ";

#[test]
fn check_rejects_changing_a_source_while_an_abs_field_views_it() {
    for (label, body, code) in [
        (
            "drop",
            "erg buf: Buffer = Buffer[4]; erg holder = ViewBuffer { held: buf, }; drop(buf);",
            "E1009",
        ),
        (
            "move",
            "erg buf: Buffer = Buffer[4]; erg holder = ViewBuffer { held: buf, }; erg other = buf;",
            "E1011",
        ),
        (
            "explicit-ref",
            "erg buf: Buffer = Buffer[4]; erg holder = ViewBuffer { held: ref buf, }; drop(buf);",
            "E1009",
        ),
        (
            "mutate-field",
            "erg plain = Plain { number: 1, }; erg holder = ViewPlain { held: plain, }; plain.number = 5;",
            "E1035",
        ),
        (
            "abs-payload",
            "erg buf: Buffer = Buffer[4]; erg holder = Option[abs Buffer].Some(buf); drop(buf);",
            "E1009",
        ),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains(code), "{label}: {stderr}");
    }
}

#[test]
fn check_rejects_storing_a_call_scope_loan_in_an_abs_field() {
    let source = format!(
        "{PRELUDE}verb store(ins buf: Buffer) {{ erg holder = ViewBuffer {{ held: buf, }}; }} verb main() {{ }}"
    );
    let output = actus("loan", "check", &source);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E1082"));
}

#[test]
fn check_accepts_a_source_once_the_holder_block_has_ended() {
    for (label, body) in [
        (
            "inner-block",
            "erg buf: Buffer = Buffer[4]; { erg holder = ViewBuffer { held: buf, }; } drop(buf);",
        ),
        (
            "read-while-viewed",
            "erg plain = Plain { number: 1, }; erg holder = ViewPlain { held: plain, }; print(plain.number);",
        ),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn executes_a_read_through_an_abs_field() {
    let source = "struct Plain { number: Int, } struct View { abs plain: Plain, } \
verb main() -> Int { erg plain = Plain { number: 42, }; erg holder = View { plain: plain, }; return holder.plain.number; }";
    let root = std::env::temp_dir().join(format!("actus-abs-field-run-{}", std::process::id()));
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

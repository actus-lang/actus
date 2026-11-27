//! A call whose result can hold a view freezes every local passed to an `abs`
//! parameter, as building the holder directly does (ADR-0046 2.3, bug 105). A
//! program that dropped, moved or mutated the source while the result still
//! viewed it used to be accepted. These tests drive the CLI so that the
//! diagnostic code and the exit status are part of the contract, and compare
//! each call with the same program that builds the holder directly.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-call-result-views-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Hv { abs b: Buffer, } struct Outer { erg inner: Hv, } \
struct Src { erg data: Buffer, } role Viewer { verb view_with(abs self: Src, abs extra: Buffer) -> Hv; } \
perform Viewer for Src { verb view_with(abs self: Src, abs extra: Buffer) -> Hv { return Hv { b: extra, }; } } \
verb make(abs src: Buffer) -> Hv { return Hv { b: src, }; } \
verb wrap(abs src: Buffer) -> Outer { return Outer { inner: Hv { b: src, }, }; } \
verb maybe(abs src: Buffer) -> Option[abs Buffer] { return Option[abs Buffer].Some(src); } \
verb measure(abs src: Buffer) -> Int { return buffer_length(buffer: abs src); } \
verb touch(ins held: Buffer) {} ";

const LOCAL: &str = "erg local: Buffer = Buffer[4]; ";

#[test]
fn check_rejects_changing_a_source_while_a_call_result_views_it() {
    for (label, body, code) in [
        ("drop", "erg h = make(src: abs local); drop(local);", "E1009"),
        ("move", "erg h = make(src: abs local); erg other = local;", "E1011"),
        ("ins-loan", "erg h = make(src: abs local); touch(held: ins local);", "E1016"),
        ("nested", "erg h = wrap(src: abs local); drop(local);", "E1009"),
        ("option", "erg h = maybe(src: abs local); drop(local);", "E1009"),
        (
            "method",
            "erg source = Src { data: Buffer[4], }; abs view = ref source; erg h = view.view_with(extra: abs local); drop(local);",
            "E1009",
        ),
        (
            "chained",
            "erg first = make(src: abs local); erg second = make(src: abs first.b); erg other = first;",
            "E1011",
        ),
    ] {
        let output = actus(label, &format!("{PRELUDE}verb main() {{ {LOCAL}{body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains(code), "{label}: {stderr}");
    }
}

#[test]
fn check_gives_the_call_the_same_error_as_the_direct_construction() {
    let call = actus(
        "parity-call",
        &format!("{PRELUDE}verb main() {{ {LOCAL}erg h = make(src: abs local); drop(local); }}"),
    );
    let direct = actus(
        "parity-direct",
        &format!("{PRELUDE}verb main() {{ {LOCAL}erg h = Hv {{ b: local, }}; drop(local); }}"),
    );
    let code = |output: &Output| {
        String::from_utf8_lossy(&output.stderr)
            .split("error[")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .map(str::to_owned)
    };
    assert_eq!(code(&call), Some("E1009".to_owned()));
    assert_eq!(code(&call), code(&direct));
}

#[test]
fn check_accepts_a_source_once_the_result_block_has_ended_or_holds_no_view() {
    for (label, body) in [
        ("inner-block", "{ erg h = make(src: abs local); } drop(local);"),
        ("inner-block-move", "{ erg h = wrap(src: abs local); } erg other = local; drop(other);"),
        ("no-view-result", "erg size = measure(src: abs local); drop(local);"),
        ("read-while-viewed", "erg h = make(src: abs local); print(measure(src: abs local));"),
    ] {
        let output = actus(label, &format!("{PRELUDE}verb main() {{ {LOCAL}{body} }}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

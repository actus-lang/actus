//! A struct, enum payload or `Option` that holds a view may leave a verb only
//! when every view in it comes from the verb's own `abs` parameters
//! (ADR-0021, ADR-0046 2.3). A view of a local used to leave through a struct
//! and dangle. These tests drive the CLI so that the diagnostic code and the
//! exit status are part of the contract, and run the forms that stay valid.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, args: &[&str], source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-escaping-views-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let mut command = Command::new(env!("CARGO_BIN_EXE_actus"));
    command.arg(args[0]).arg(&input).args(&args[1..]);
    let output = command.output().expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Hv { abs b: Buffer, } struct Holder { erg kept: Hv, } \
verb make(abs src: Buffer) -> Hv { return Hv { b: src, }; } ";

#[test]
fn check_rejects_returning_a_view_of_a_local_inside_an_aggregate() {
    for (label, declarations) in [
        ("literal", "verb f() -> Hv { erg buf: Buffer = Buffer[4]; return Hv { b: buf, }; }"),
        (
            "binding",
            "verb f() -> Hv { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; return h; }",
        ),
        (
            "option",
            "verb f() -> Option[abs Buffer] { erg buf: Buffer = Buffer[4]; return Option[abs Buffer].Some(buf); }",
        ),
        (
            "result",
            "verb f() -> Result[Hv, Int] { erg buf: Buffer = Buffer[4]; return Result[Hv, Int].Ok(Hv { b: buf, }); }",
        ),
        ("owned-parameter", "verb f(erg own: Buffer) -> Hv { return Hv { b: own, }; }"),
        ("call", "verb f() -> Hv { erg local: Buffer = Buffer[4]; return make(src: abs local); }"),
    ] {
        let output =
            actus(label, &["check"], &format!("{PRELUDE}{declarations} verb main() {{ }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1018"), "{label}: {stderr}");
        assert!(stderr.contains("cannot escape its scope"), "{label}: {stderr}");
    }
}

#[test]
fn check_rejects_storing_a_view_of_a_local_through_an_ins_parameter() {
    for (label, declarations) in [
        (
            "field",
            "verb f(ins out: Holder) { erg buf: Buffer = Buffer[4]; out.kept = Hv { b: buf, }; }",
        ),
        (
            "binding",
            "verb f(ins out: Holder) { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; out.kept = h; }",
        ),
    ] {
        let output =
            actus(label, &["check"], &format!("{PRELUDE}{declarations} verb main() {{ }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1018"), "{label}: {stderr}");
    }
}

#[test]
fn check_accepts_views_of_abs_parameters_and_views_that_stay_in_the_verb() {
    for (label, declarations) in [
        ("literal", "verb f(abs src: Buffer) -> Hv { return Hv { b: src, }; }"),
        ("binding", "verb f(abs src: Buffer) -> Hv { erg h = Hv { b: src, }; return h; }"),
        (
            "result",
            "verb f(abs src: Buffer) -> Result[Hv, Int] { return Result[Hv, Int].Ok(Hv { b: src, }); }",
        ),
        ("ins-store", "verb f(ins out: Holder, abs src: Buffer) { out.kept = Hv { b: src, }; }"),
        (
            "dat-parameter",
            "verb sink(dat held: Hv) { } verb f() { erg buf: Buffer = Buffer[4]; erg h = Hv { b: buf, }; sink(held: dat h); }",
        ),
    ] {
        let output =
            actus(label, &["check"], &format!("{PRELUDE}{declarations} verb main() {{ }}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn executes_a_returned_aggregate_that_views_an_abs_parameter() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg data: Buffer = Buffer[4]; erg h = make(src: abs data); \
return buffer_length(buffer: abs h.b); }}"
    );
    let output =
        std::env::temp_dir().join(format!("actus-escaping-views-run-{}", std::process::id()));
    let build = actus("run", &["build", "--emit", "exe", "-o", output.to_str().unwrap()], &source);
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let status = Command::new(&output).status().expect("run fixture");
    assert_eq!(status.code(), Some(4));
    let _ = fs::remove_file(output);
}

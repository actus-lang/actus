//! A view that a `case` takes of its subject ends with the case (ADR-0016).
//! These tests drive the CLI so that the diagnostic code and the exit status
//! are part of the contract.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, command: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-case-view-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([command, input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Cfg { erg a: Buffer, erg b: Option[Buffer], } \
verb one(abs x: Buffer) {} verb f1(dat c: Cfg) { drop(c); } ";

#[test]
fn check_accepts_drop_after_a_case_whose_arm_viewed_another_field() {
    let source = format!(
        "{PRELUDE}verb f3(dat c: Cfg) {{ case c.b {{ Option.None => {{}}, \
         Option.Some(inner) => {{ one(x: abs c.a); }}, }}; drop(c); }} \
         verb main() -> Int {{ return 0; }}"
    );
    let output = actus("accepted", "check", &source);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn check_rejects_drop_inside_the_case_that_views_the_owner() {
    let source = format!(
        "{PRELUDE}verb f3(dat c: Cfg) {{ case c.b {{ Option.None => {{}}, \
         Option.Some(inner) => {{ drop(c); }}, }}; }} verb main() -> Int {{ return 0; }}"
    );
    let output = actus("rejected", "check", &source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("E1009"), "{stderr}");
}

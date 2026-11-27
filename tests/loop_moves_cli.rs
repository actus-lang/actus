//! A value that is declared before a loop and moved inside it is moved again
//! by the next iteration, which used to free the same buffer twice (ADR-0016).
//! These tests drive the CLI so that the diagnostic code, the message, the
//! location and the exit status are part of the contract, and run the forms
//! that stay valid.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, args: &[&str], source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-loop-moves-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let mut command = Command::new(env!("CARGO_BIN_EXE_actus"));
    command.arg(args[0]).arg(&input).args(&args[1..]);
    let output = command.output().expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Holder { held: Buffer, } enum Slot { Full(Buffer), Empty, } \
verb eat(dat packet: Buffer) { drop(packet); } ";

#[test]
fn check_rejects_a_move_that_the_next_iteration_repeats() {
    for (label, body) in [
        (
            "loop",
            "erg x: Buffer = Buffer[4]; erg n: Int = 0; loop { if n > 2 { break; } eat(packet: x); n += 1; }",
        ),
        ("for-range", "erg x: Buffer = Buffer[4]; for erg i in 0..3 { eat(packet: x); }"),
        (
            "drop",
            "erg x: Buffer = Buffer[4]; erg n: Int = 0; loop { if n > 2 { break; } drop(x); n += 1; }",
        ),
        (
            "nested",
            "erg x: Buffer = Buffer[4]; for erg i in 0..2 { for erg j in 0..2 { eat(packet: x); } }",
        ),
        (
            "struct-field",
            "erg x: Buffer = Buffer[4]; for erg i in 0..2 { erg h = Holder { held: x, }; drop(h); }",
        ),
        (
            "continue",
            "erg x: Buffer = Buffer[4]; erg n: Int = 0; loop { n += 1; if n > 3 { break; } if n > 1 { eat(packet: x); continue; } }",
        ),
        (
            "enum-payload",
            "erg x: Buffer = Buffer[4]; for erg i in 0..2 { erg s = Slot.Full(x); drop(s); }",
        ),
    ] {
        let output = actus(label, &["check"], &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1005"), "{label}: {stderr}");
        assert!(stderr.contains("moved in a previous loop iteration"), "{label}: {stderr}");
    }
}

#[test]
fn check_reports_the_move_not_a_later_use() {
    let source = format!(
        "{PRELUDE}verb main() {{\n    erg x: Buffer = Buffer[4];\n    for erg i in 0..3 {{\n        eat(packet: x);\n    }}\n}}"
    );
    let output = actus("location", &["check"], &source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("at 4:21"), "{stderr}");
}

#[test]
fn check_accepts_moves_that_leave_the_loop_or_belong_to_the_body() {
    for (label, body) in [
        ("break", "erg x: Buffer = Buffer[4]; loop { eat(packet: x); break; }"),
        (
            "break-in-if",
            "erg x: Buffer = Buffer[4]; erg n: Int = 0; loop { n += 1; if n > 2 { eat(packet: x); break; } }",
        ),
        ("return", "erg x: Buffer = Buffer[4]; loop { eat(packet: x); return; }"),
        ("for-break", "erg x: Buffer = Buffer[4]; for erg i in 0..3 { eat(packet: x); break; }"),
        ("declared-inside", "for erg i in 0..3 { erg y: Buffer = Buffer[4]; eat(packet: y); }"),
        (
            "declared-inside-nested",
            "for erg i in 0..3 { erg y: Buffer = Buffer[4]; for erg j in 0..3 { append(y, 1u8); } drop(y); }",
        ),
    ] {
        let output = actus(label, &["check"], &format!("{PRELUDE}verb main() {{ {body} }}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn executes_a_loop_that_moves_a_value_and_leaves() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg x: Buffer = Buffer[4]; erg n: Int = 0; \
loop {{ n += 1; if n > 2 {{ eat(packet: x); break; }} }} return n; }}"
    );
    let output = std::env::temp_dir().join(format!("actus-loop-moves-run-{}", std::process::id()));
    let build = actus("run", &["build", "--emit", "exe", "-o", output.to_str().unwrap()], &source);
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let status = Command::new(&output).status().expect("run fixture");
    assert_eq!(status.code(), Some(3));
    let _ = fs::remove_file(output);
}

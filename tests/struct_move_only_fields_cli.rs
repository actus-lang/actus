//! A plain struct field that receives an arena or a buffer takes ownership of
//! it exactly like an `erg` field (ADR-0030, ADR-0046), so the source cannot be
//! used afterwards. These tests drive the CLI so that the diagnostic code and
//! the exit status are part of the contract.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn check(label: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-move-only-field-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().unwrap()])
        .output()
        .expect("start check");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str = "struct Node { value: Int, } struct PlainArena { held: Arena[64], } \
struct PlainBuffer { held: Buffer, } struct Inner { number: Int, } struct PlainStruct { held: Inner, } ";

#[test]
fn check_rejects_use_of_a_source_that_a_plain_field_took() {
    for (label, body) in [
        (
            "arena-place",
            "erg arena: Arena[64] = Arena[64](); erg holder = PlainArena { held: arena, }; erg node = arena.place(value: Node { value: 1, });",
        ),
        (
            "buffer-append",
            "erg buffer: Buffer = Buffer[4]; erg holder = PlainBuffer { held: buffer, }; append(buffer, 1u8);",
        ),
        (
            "struct-read",
            "erg inner = Inner { number: 1, }; erg holder = PlainStruct { held: inner, }; print(inner.number);",
        ),
    ] {
        let output = check(label, &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1005"), "{label}: {stderr}");
    }
}

#[test]
fn check_rejects_a_plain_field_that_takes_an_arena_while_a_view_is_live() {
    let body = "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); abs view = ref node; erg holder = PlainArena { held: arena, }; print(view.value);";
    let output = check("live-view", &format!("{PRELUDE}verb main() {{ {body} }}"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E1079"));
}

#[test]
fn check_accepts_a_plain_field_that_takes_a_resource_nobody_reads_again() {
    for (label, body) in [
        (
            "arena",
            "erg arena: Arena[64] = Arena[64](); erg holder = PlainArena { held: arena, }; print(1);",
        ),
        (
            "buffer",
            "erg buffer: Buffer = Buffer[4]; erg holder = PlainBuffer { held: buffer, }; print(1);",
        ),
        ("fresh", "erg holder = PlainArena { held: Arena[64](), }; print(1);"),
    ] {
        let output = check(label, &format!("{PRELUDE}verb main() {{ {body} }}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

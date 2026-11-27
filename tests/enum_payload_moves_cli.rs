//! An enum payload that receives an arena or a buffer takes ownership of it
//! (ADR-0030, ADR-0059), so the source cannot be used afterwards. These tests
//! drive the CLI so that the diagnostic code and the exit status are part of
//! the contract; the accepted forms are also built and run.
#![cfg(unix)]

use std::fs;
use std::process::{Command, Output};

fn actus(label: &str, command: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-enum-payload-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([command, input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

const PRELUDE: &str =
    "struct Node { value: Int, } enum Mine { Arena(Arena[64]), Buffer(Buffer), } ";

#[test]
fn check_rejects_use_of_a_source_that_a_payload_took() {
    for (label, body, code) in [
        (
            "option-arena-place",
            "erg arena: Arena[64] = Arena[64](); erg held: Option[Arena[64]] = Option[Arena[64]].Some(arena); erg node = arena.place(value: Node { value: 1, });",
            "E1005",
        ),
        (
            "option-arena-drop",
            "erg arena: Arena[64] = Arena[64](); erg held = Option[Arena[64]].Some(arena); drop(arena);",
            "E1007",
        ),
        (
            "option-buffer",
            "erg buffer: Buffer = Buffer[4]; erg held = Option[Buffer].Some(buffer); append(buffer, 1u8);",
            "E1005",
        ),
        (
            "result-arena",
            "erg arena: Arena[64] = Arena[64](); erg held = Result[Arena[64], Int].Ok(arena); drop(arena);",
            "E1007",
        ),
        (
            "user-arena",
            "erg arena: Arena[64] = Arena[64](); erg held = Mine.Arena(arena); drop(arena);",
            "E1007",
        ),
        (
            "nested",
            "erg buffer: Buffer = Buffer[4]; erg inner = Option[Buffer].Some(buffer); erg held = Option[Option[Buffer]].Some(inner); drop(inner);",
            "E1007",
        ),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains(code), "{label}: {stderr}");
    }
}

#[test]
fn check_rejects_an_arena_that_moves_into_a_payload_while_a_view_is_live() {
    let body = "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); abs view = ref node; erg held = Option[Arena[64]].Some(arena); print(view.value);";
    let output = actus("live-view", "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E1079"));
}

#[test]
fn check_accepts_a_payload_that_takes_a_resource_nobody_reads_again() {
    for (label, body) in [
        (
            "arena",
            "erg arena: Arena[64] = Arena[64](); erg held = Option[Arena[64]].Some(arena); print(1);",
        ),
        ("buffer", "erg buffer: Buffer = Buffer[4]; erg held = Mine.Buffer(buffer); print(1);"),
        ("fresh", "erg held = Option[Arena[64]].Some(Arena[64]()); print(1);"),
        ("scalar", "erg count: Int = 1; erg held = Option[Int].Some(count); print(count);"),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn executes_an_arena_and_a_buffer_moved_into_payloads() {
    for (label, source, expected) in [
        (
            "arena",
            "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 42, }); erg answer: Int = node.value; erg held: Option[Arena[64]] = Option[Arena[64]].Some(arena); return answer; }",
            42,
        ),
        (
            "buffer",
            "verb main() -> Int { erg source: Buffer = Buffer[4]; append(source, 42u8); erg held: Option[Buffer] = Option[Buffer].Some(source); return case dat held { Option.Some(inner) => inner[4u32] as Int, Option.None => 1, }; }",
            42,
        ),
    ] {
        let root = std::env::temp_dir()
            .join(format!("actus-enum-payload-run-{label}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write fixture");
        let build = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args([
                "build",
                input.to_str().unwrap(),
                "--emit",
                "exe",
                "-o",
                output.to_str().unwrap(),
            ])
            .output()
            .expect("start build");
        assert!(build.status.success(), "{label}: {}", String::from_utf8_lossy(&build.stderr));
        let status = Command::new(&output).status().expect("run fixture");
        assert_eq!(status.code(), Some(expected), "{label}");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

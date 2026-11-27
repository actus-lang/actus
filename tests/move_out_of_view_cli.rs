//! Bug #108: reading a struct, enum, pack or array field BY VALUE through an
//! `abs` view or an `ins` loan is rejected (the view is not an owner, so the
//! value would be shared silently), and the written
//! `copy(value: abs path)` of a copy type is a value of its own on every back
//! end that is built in: changing the copy leaves the original unchanged.
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

const BACKENDS_BUILT: usize =
    cfg!(feature = "llvm") as usize + cfg!(feature = "cranelift") as usize;

fn actus(label: &str, command: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-view-reads-{label}-{}", std::process::id()))
        .with_extension("act");
    fs::write(&input, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([command, input.to_str().unwrap()])
        .output()
        .expect("start actus");
    let _ = fs::remove_file(input);
    output
}

fn run_on_every_backend(label: &str, source: &str) -> usize {
    let mut ran = 0;
    for backend in [LLVM, CRANELIFT].into_iter().flatten() {
        let root = std::env::temp_dir()
            .join(format!("actus-view-reads-{label}-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write fixture");
        let build = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["build", input.to_str().unwrap(), "--backend", backend, "--emit", "exe", "-o"])
            .arg(&output)
            .output()
            .expect("start the build");
        assert!(
            build.status.success(),
            "{label} on {backend}: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run the program");
        assert_eq!(status.code(), Some(42), "{label} on {backend}");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
        ran += 1;
    }
    ran
}

const PRELUDE: &str = "meta copy struct Stamp { kind: u8, length: u32, } \
struct Plain { kind: u8, } enum Mode { Idle, Busy, } \
meta copy pack Cell { erg storage: u16; layout little; fields { erg kind: u8 at 0; erg size: u8 at 8; } } \
struct Parser { erg stamp: Stamp, erg plain: Plain, erg cell: Cell, erg mode: Mode, \
erg stamps: Array[Stamp, 2], erg bytes: Array[u8, 4], count: Int, } ";

fn check(label: &str, body: &str) -> (bool, String) {
    let output = actus(label, "check", &format!("{PRELUDE}{body} verb main() {{ }}"));
    (output.status.success(), String::from_utf8_lossy(&output.stderr).into_owned())
}

#[test]
fn check_rejects_a_field_read_by_value_through_an_abs_view() {
    for (label, body, path, fix) in [
        ("struct", "verb get(abs p: Parser) -> Stamp { return p.stamp; }", "p.stamp", "copy"),
        ("plain", "verb get(abs p: Parser) -> Plain { return p.plain; }", "p.plain", "borrow"),
        ("pack", "verb get(abs p: Parser) -> Cell { return p.cell; }", "p.cell", "copy"),
        ("enum", "verb get(abs p: Parser) -> Mode { return p.mode; }", "p.mode", "borrow"),
        (
            "array",
            "verb get(abs p: Parser) -> Array[Stamp, 2] { return p.stamps; }",
            "p.stamps",
            "copy",
        ),
        (
            "element",
            "verb get(abs p: Parser) -> Stamp { return p.stamps[1]; }",
            "p.stamps[1]",
            "copy",
        ),
        ("init", "verb get(abs p: Parser) { erg x = p.plain; }", "p.plain", "borrow"),
    ] {
        let (accepted, stderr) = check(label, body);
        assert!(!accepted, "{label}: must be rejected");
        assert!(stderr.contains("E1010"), "{label}: {stderr}");
        assert!(stderr.contains(&format!("`{path}`")), "{label}: {stderr}");
        let expected = if fix == "copy" {
            format!("copy(value: abs {path})")
        } else {
            "take ownership of the owner".to_owned()
        };
        assert!(stderr.contains(&expected), "{label}: {stderr}");
    }
}

#[test]
fn check_rejects_a_field_read_by_value_through_an_ins_loan() {
    let (accepted, stderr) = check("ins", "verb get(ins p: Parser) -> Stamp { return p.stamp; }");
    assert!(!accepted, "must be rejected");
    assert!(stderr.contains("E1082"), "{stderr}");
    assert!(stderr.contains("`p.stamp`"), "{stderr}");
}

#[test]
fn check_accepts_the_written_copy_a_scalar_read_and_a_view_of_a_field() {
    for (label, body) in [
        ("copy", "verb get(abs p: Parser) -> Stamp { return copy(value: abs p.stamp); }"),
        ("copy-array", "verb get(abs p: Parser) { erg x = copy(value: abs p.stamps); }"),
        ("scalar", "verb get(abs p: Parser) -> Int { return p.count; }"),
        ("nested-scalar", "verb get(abs p: Parser) -> u32 { return p.stamp.length; }"),
        ("ref", "verb get(abs p: Parser) { abs h = ref p.stamp; }"),
        ("abs-return", "verb get(abs p: Parser) -> abs Stamp { return p.stamp; }"),
    ] {
        let (accepted, stderr) = check(label, body);
        assert!(accepted, "{label}: {stderr}");
    }
}

#[test]
fn executes_a_copy_of_a_struct_field_read_through_a_view() {
    let source = format!(
        "{PRELUDE}verb stamp_of(abs p: Parser) -> Stamp {{ return copy(value: abs p.stamp); }} \
        verb main() -> Int {{ erg parser = Parser {{ stamp: Stamp {{ kind: 1u8, length: 40u32, }}, \
        plain: Plain {{ kind: 0u8, }}, cell: Cell {{ storage: 1u16, }}, mode: Mode.Idle, \
        stamps: Array[Stamp, 2](), bytes: Array[u8, 4](), count: 0, }}; \
        erg duplicate = stamp_of(p: abs parser); duplicate.length = 41u32; duplicate.length += 1u32; \
        if parser.stamp.length != 40u32 {{ return 1; }} \
        return (duplicate.length as Int) + (parser.stamp.length as Int) - 40; }}"
    );
    assert_eq!(run_on_every_backend("view-struct", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_copy_of_a_pack_and_an_array_field_read_through_a_view() {
    let source = format!(
        "{PRELUDE}verb cell_of(abs p: Parser) -> Cell {{ return copy(value: abs p.cell); }} \
        verb bytes_of(abs p: Parser) -> Array[u8, 4] {{ return copy(value: abs p.bytes); }} \
        verb main() -> Int {{ erg parser = Parser {{ stamp: Stamp {{ kind: 1u8, length: 40u32, }}, \
        plain: Plain {{ kind: 0u8, }}, cell: Cell {{ storage: 1u16, }}, mode: Mode.Idle, \
        stamps: Array[Stamp, 2](), bytes: Array[u8, 4](), count: 0, }}; \
        parser.bytes[0] = 9u8; \
        erg cell = cell_of(p: abs parser); cell.kind = 33u8; \
        erg bytes = bytes_of(p: abs parser); bytes[0] = 7u8; \
        if parser.cell.kind != 1u8 {{ return 1; }} if parser.bytes[0] != 9u8 {{ return 2; }} \
        return (cell.kind as Int) + (bytes[0] as Int) + (parser.bytes[0] as Int) - 7; }}"
    );
    assert_eq!(run_on_every_backend("view-pack-array", &source), BACKENDS_BUILT);
}

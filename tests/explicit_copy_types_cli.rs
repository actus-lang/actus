//! No struct or enum is duplicated implicitly (ADR-0124). These tests drive the
//! CLI so that the diagnostic codes and the exit status are part of the
//! contract, and run the written `copy(value: abs x)` of a `meta copy` type on
//! every back end that is built in: the copy is a value of its own, so
//! changing it leaves the original unchanged.
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
        .join(format!("actus-copy-types-{label}-{}", std::process::id()))
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
            .join(format!("actus-copy-types-{label}-{backend}-{}", std::process::id()));
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

const PRELUDE: &str = "struct Inner { number: Int, } struct Holder { held: Inner, } \
enum Mode { Idle, Busy, } struct Slot { mode: Mode, } \
verb take(erg held: Inner) {} verb take_mode(erg mode: Mode) {} ";

#[test]
fn check_rejects_use_of_a_struct_or_an_enum_after_it_moved() {
    for (label, body) in [
        ("struct-binding", "erg a = Inner { number: 1, }; erg b = a; print(a.number);"),
        (
            "struct-field",
            "erg a = Inner { number: 1, }; erg h = Holder { held: a, }; print(a.number);",
        ),
        (
            "struct-field-assignment",
            "erg a = Inner { number: 1, }; erg h = Holder { held: Inner { number: 2, }, }; h.held = a; print(a.number);",
        ),
        (
            "struct-payload",
            "erg a = Inner { number: 1, }; erg o = Option[Inner].Some(a); print(a.number);",
        ),
        ("struct-argument", "erg a = Inner { number: 1, }; take(held: a); print(a.number);"),
        ("enum-binding", "erg a = Mode.Idle; erg b = a; if a == Mode.Idle { print(1); }"),
        (
            "enum-field",
            "erg a = Mode.Idle; erg s = Slot { mode: a, }; if a == Mode.Idle { print(1); }",
        ),
        (
            "enum-payload",
            "erg a = Mode.Idle; erg o = Option[Mode].Some(a); if a == Mode.Idle { print(1); }",
        ),
        ("enum-argument", "erg a = Mode.Idle; take_mode(mode: a); if a == Mode.Idle { print(1); }"),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1005"), "{label}: {stderr}");
    }
}

#[test]
fn check_accepts_reads_of_a_struct_and_an_enum() {
    let body = "erg a = Inner { number: 1, }; erg m = Mode.Idle; print(a.number); print(a.number); \
        if m == Mode.Idle { print(1); } if m != Mode.Busy { print(2); } \
        erg v: Int = case abs m { Mode.Idle => 1, Mode.Busy => 2, }; print(v);";
    let output = actus("reads", "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn check_accepts_a_written_copy_of_a_meta_copy_type() {
    let source = "meta copy struct Stamp { tick: u32, } meta copy enum Level { Low, High, } \
        verb main() { erg s = Stamp { tick: 1u32, }; erg c = copy(value: abs s); \
        erg l = Level.Low; erg d = copy(value: abs l); \
        print(int: s.tick); print(int: c.tick); if l == Level.Low { print(1); } }";
    let output = actus("written-copy", "check", source);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn check_rejects_a_copy_of_a_type_that_is_not_a_copy_type() {
    for (label, body) in [
        ("plain-struct", "erg a = Inner { number: 1, }; erg b = copy(value: abs a);"),
        ("plain-enum", "erg a = Mode.Idle; erg b = copy(value: abs a);"),
        ("buffer", "erg a: Buffer = Buffer[4]; erg b = copy(value: abs a);"),
    ] {
        let output = actus(label, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1021"), "{label}: {stderr}");
    }
}

#[test]
fn check_rejects_meta_copy_over_a_field_that_is_not_a_copy_type() {
    let support = "struct Plain { tick: u32, } enum PlainMode { Idle, } \
        struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } ";
    for (label, declaration) in [
        ("buffer", "meta copy struct Frame { n: Int, held: Buffer, }"),
        ("arena", "meta copy struct Frame { n: Int, held: Arena[64], }"),
        ("abs-view", "meta copy struct Frame { n: Int, abs held: Plain, }"),
        ("struct", "meta copy struct Frame { n: Int, held: Plain, }"),
        ("enum", "meta copy struct Frame { n: Int, held: PlainMode, }"),
        ("drop-type", "meta copy struct Frame { n: Int, held: Counter, }"),
        ("enum-payload", "meta copy enum Frame { Empty, Held(Buffer), }"),
    ] {
        let output = actus(label, "check", &format!("{support}{declaration} verb main() {{}}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1146"), "{label}: {stderr}");
        assert!(stderr.contains("`Frame`"), "{label}: {stderr}");
        assert!(stderr.to_lowercase().contains("held"), "{label}: {stderr}");
    }
}

#[test]
fn check_accepts_meta_copy_over_copy_types() {
    let source = "meta copy struct Stamp { tick: u32, } meta copy enum Level { Low, High(u8), } \
        meta copy struct Frame { n: Int, stamp: Stamp, level: Level, values: Array[Stamp, 2], } \
        verb main() {}";
    let output = actus("accepted", "check", source);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn check_rejects_meta_copy_on_other_declarations_and_twice() {
    for (label, source, code) in [
        ("verb", "meta copy verb run() { } verb main() {}", "E0008"),
        ("twice", "meta copy meta copy struct Stamp { tick: u32, } verb main() {}", "E0013"),
    ] {
        let output = actus(label, "check", source);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains(code), "{label}: {stderr}");
    }
}

const STAMP: &str = "meta copy struct Stamp { kind: u8, length: u32, } \
meta copy struct Frame { stamp: Stamp, values: Array[u8, 4], } struct Holder { stamp: Stamp, } \
meta copy enum Level { Low, High(u8), } \
verb bump(erg stamp: Stamp) -> u32 { stamp.length += 2u32; return stamp.length; } ";

#[test]
fn executes_a_copy_that_changes_without_changing_the_original() {
    let source = format!(
        "{STAMP}verb main() -> Int {{ erg original = Stamp {{ kind: 1u8, length: 40u32, }}; \
        erg duplicate = copy(value: abs original); duplicate.length = 41u32; duplicate.length += 1u32; \
        if original.length != 40u32 {{ return 1; }} \
        return (duplicate.length as Int) + (original.length as Int) - 40; }}"
    );
    assert_eq!(run_on_every_backend("independent", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_copy_written_into_a_struct_field() {
    let source = format!(
        "{STAMP}verb main() -> Int {{ erg original = Stamp {{ kind: 1u8, length: 40u32, }}; \
        erg holder = Holder {{ stamp: copy(value: abs original), }}; holder.stamp.length = 2u32; \
        if original.length != 40u32 {{ return 1; }} \
        return (holder.stamp.length as Int) + (original.length as Int); }}"
    );
    assert_eq!(run_on_every_backend("field", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_copy_given_to_an_erg_parameter() {
    let source = format!(
        "{STAMP}verb main() -> Int {{ erg original = Stamp {{ kind: 1u8, length: 40u32, }}; \
        erg lent = copy(value: abs original); erg bumped = bump(stamp: lent); \
        if original.length != 40u32 {{ return 1; }} \
        return (bumped as Int) + (original.length as Int) - 40; }}"
    );
    assert_eq!(run_on_every_backend("argument", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_copy_of_a_struct_that_holds_an_array() {
    let source = format!(
        "{STAMP}verb main() -> Int {{ erg frame = Frame {{ stamp: Stamp {{ kind: 1u8, length: 40u32, }}, values: Array[u8, 4](), }}; \
        frame.values[0] = 9u8; erg other = copy(value: abs frame); other.values[0] = 33u8; other.stamp.length = 2u32; \
        if frame.values[0] != 9u8 {{ return 1; }} if frame.stamp.length != 40u32 {{ return 2; }} \
        return (other.values[0] as Int) + (frame.values[0] as Int); }}"
    );
    assert_eq!(run_on_every_backend("array", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_copy_of_an_enum_with_a_payload() {
    let source = format!(
        "{STAMP}verb main() -> Int {{ erg level = Level.High(40u8); erg level_copy = copy(value: abs level); \
        erg seen: Int = case dat level_copy {{ Level.Low => 0, Level.High(n) => n as Int, }}; \
        erg again: Int = case dat level {{ Level.Low => 0, Level.High(n) => n as Int, }}; \
        if seen != again {{ return 1; }} return seen + 2; }}"
    );
    assert_eq!(run_on_every_backend("enum", &source), BACKENDS_BUILT);
}

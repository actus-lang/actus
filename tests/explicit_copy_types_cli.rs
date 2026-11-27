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

const CELL: &str = "pack Cell { erg storage: u16; layout little; fields { erg kind: u8 at 0; erg size: u8 at 8; } } \
struct Holder { held: Cell, } verb take(erg held: Cell) {} ";

#[test]
fn check_rejects_use_of_a_pack_after_it_moved() {
    for (label, body) in [
        ("binding", "erg a = Cell { storage: 1u16, }; erg b = a; print(int: a.kind);"),
        (
            "field",
            "erg a = Cell { storage: 1u16, }; erg h = Holder { held: a, }; print(int: a.kind);",
        ),
        (
            "payload",
            "erg a = Cell { storage: 1u16, }; erg o = Option[Cell].Some(a); print(int: a.kind);",
        ),
        ("argument", "erg a = Cell { storage: 1u16, }; take(held: a); print(int: a.kind);"),
    ] {
        let output = actus(label, "check", &format!("{CELL}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: must be rejected");
        assert!(stderr.contains("E1005"), "{label}: {stderr}");
    }
}

#[test]
fn check_accepts_reads_and_a_written_copy_of_a_meta_copy_pack() {
    let source = "meta copy pack Cell { erg storage: u16; layout little; fields { erg kind: u8 at 0; erg size: u8 at 8; } } \
        verb main() { erg a = Cell { storage: 1u16, }; print(int: a.kind); print(int: a.kind + a.size); \
        erg b = copy(value: abs a); print(int: a.kind); print(int: b.kind); }";
    let output = actus("pack-copy", "check", source);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn check_rejects_a_copy_of_a_pack_that_is_not_meta_copy() {
    let output = actus(
        "pack-plain-copy",
        "check",
        &format!(
            "{CELL}verb main() {{ erg a = Cell {{ storage: 1u16, }}; erg b = copy(value: abs a); }}"
        ),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "must be rejected");
    assert!(stderr.contains("E1021"), "{stderr}");
}

#[test]
fn check_rejects_meta_copy_on_a_pack_with_a_drop_contract() {
    let source = "meta copy pack Cell { erg storage: u8; layout little; fields { erg kind: u8 at 0; } } \
        perform Drop for Cell { verb drop(ins self: Cell) { } } verb main() {}";
    let output = actus("pack-drop", "check", source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "must be rejected");
    assert!(stderr.contains("E1146") && stderr.contains("`Cell`"), "{stderr}");
}

const PACKS: &str = "meta copy pack Register { erg storage: u32; layout little; fields { \
erg enabled: u1 at 0; abs mode: u3 at 1; erg channel: u5 at 4; abs _reserved: u23 at 9 = 0; } } \
meta copy pack Header { erg storage: Array[u8, 4]; layout little; fields { \
erg kind: u8 at 0; erg length: u16 at 8; erg flag: u8 at 24; } } \
struct Slot { register: Register, } verb bump(erg register: Register) -> u8 { register.channel = 31; return register.channel as u8; } ";

#[test]
fn executes_a_pack_copy_that_changes_without_changing_the_original() {
    let source = format!(
        "{PACKS}verb main() -> Int {{ erg original = Register {{ storage: 0u32, }}; original.channel = 3; \
        erg duplicate = copy(value: abs original); duplicate.channel = 9; duplicate.enabled = 1; \
        if original.channel != 3 {{ return 1; }} if original.enabled != 0 {{ return 2; }} \
        return (duplicate.channel as Int) + (duplicate.enabled as Int) + (original.channel as Int) + 29; }}"
    );
    assert_eq!(run_on_every_backend("pack-independent", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_copy_of_an_array_backed_pack_that_changes_without_changing_the_original() {
    let source = format!(
        "{PACKS}verb main() -> Int {{ erg original = Header {{ storage: Array[u8, 4](), }}; \
        original.kind = 5u8; original.length = 300u16; original.flag = 1u8; \
        erg duplicate = copy(value: abs original); duplicate.kind = 7u8; duplicate.length = 1u16; duplicate.flag = 9u8; \
        if original.kind != 5u8 {{ return 1; }} if original.length != 300u16 {{ return 2; }} \
        if original.flag != 1u8 {{ return 3; }} \
        return (duplicate.kind as Int) + (duplicate.length as Int) + (duplicate.flag as Int) + 25; }}"
    );
    assert_eq!(run_on_every_backend("pack-array-independent", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_pack_copy_written_into_a_struct_field_and_an_erg_parameter() {
    let source = format!(
        "{PACKS}verb main() -> Int {{ erg original = Register {{ storage: 0u32, }}; original.channel = 3; \
        erg slot = Slot {{ register: copy(value: abs original), }}; slot.register.channel = 4; \
        erg lent = copy(value: abs original); erg bumped = bump(register: lent); \
        if original.channel != 3 {{ return 1; }} \
        return (slot.register.channel as Int) + (bumped as Int) + (original.channel as Int) + 4; }}"
    );
    assert_eq!(run_on_every_backend("pack-field", &source), BACKENDS_BUILT);
}

#[test]
fn executes_a_copy_of_a_pack_stored_in_an_array_element() {
    let source = format!(
        "{PACKS}verb main() -> Int {{ erg original = Header {{ storage: Array[u8, 4](), }}; original.kind = 40u8; \
        erg items: Array[Header, 2] = Array[Header, 2](); items[1] = copy(value: abs original); \
        items[1].kind = 2u8; if original.kind != 40u8 {{ return 1; }} \
        return (items[1].kind as Int) + (original.kind as Int); }}"
    );
    assert_eq!(run_on_every_backend("pack-element", &source), BACKENDS_BUILT);
}

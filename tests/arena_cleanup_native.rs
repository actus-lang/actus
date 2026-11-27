//! Native execution of programs that give an arena away before its scope ends
//! (ADR-0027, ADR-0030). Only the live owner is cleaned up, so every program
//! returns the sum of its placed values on each back end that is built in.
#![cfg(unix)]

use std::fs;
use std::process::Command;

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

fn run_on_every_backend(label: &str, source: &str) -> usize {
    let mut ran = 0;
    for backend in [LLVM, CRANELIFT].into_iter().flatten() {
        let root = std::env::temp_dir()
            .join(format!("actus-arena-cleanup-{label}-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write arena fixture");
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

const PRELUDE: &str = "struct Node { value: Int, } struct Holder { held: Arena[64], } \
verb take(dat held: Arena[64]) {} verb lend(ins held: Arena[64]) {} \
verb pass(dat held: Arena[64]) -> Arena[64] { return held; } ";

#[test]
fn keeps_placed_values_intact_after_a_loan_and_a_move_into_a_callee() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg arena: Arena[64] = Arena[64](); erg first = arena.place(value: Node {{ value: 20, }}); lend(held: ins arena); erg second = arena.place(value: Node {{ value: 22, }}); erg total = first.value + second.value; take(held: arena); return total; }}"
    );
    assert_eq!(run_on_every_backend("callee", &source), BACKENDS_BUILT);
}

#[test]
fn keeps_placed_values_intact_after_a_move_into_a_struct_field() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg arena: Arena[64] = Arena[64](); erg first = arena.place(value: Node {{ value: 40, }}); erg total = first.value; erg holder = Holder {{ held: arena, }}; return total + 2; }}"
    );
    assert_eq!(run_on_every_backend("field", &source), BACKENDS_BUILT);
}

#[test]
fn places_through_the_binding_an_arena_moved_into() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg arena: Arena[64] = Arena[64](); erg other = arena; erg first = other.place(value: Node {{ value: 20, }}); erg second = other.place(value: Node {{ value: 22, }}); return first.value + second.value; }}"
    );
    assert_eq!(run_on_every_backend("binding", &source), BACKENDS_BUILT);
}

#[test]
fn places_through_the_arena_a_callee_returned() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg arena: Arena[64] = Arena[64](); erg back: Arena[64] = pass(held: arena); erg first = back.place(value: Node {{ value: 20, }}); erg second = back.place(value: Node {{ value: 22, }}); return first.value + second.value; }}"
    );
    assert_eq!(run_on_every_backend("returned", &source), BACKENDS_BUILT);
}

#[test]
fn places_in_a_nested_scope_that_gives_the_arena_away() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg total: Int = 0; {{ erg arena: Arena[64] = Arena[64](); erg first = arena.place(value: Node {{ value: 20, }}); erg second = arena.place(value: Node {{ value: 22, }}); total = first.value + second.value; take(held: arena); }} return total; }}"
    );
    assert_eq!(run_on_every_backend("nested", &source), BACKENDS_BUILT);
}

//! `copy(value: abs xs)` of an `Array` of copy types (ADR-0124) is checked
//! through the CLI, and the copy is run on every back end that is built in: it
//! has storage of its own, so changing it leaves the original unchanged.
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

fn actus(label: &str, command: &str, source: &str) -> Output {
    let input = std::env::temp_dir()
        .join(format!("actus-array-copy-{label}-{}", std::process::id()))
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
            .join(format!("actus-array-copy-{label}-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let binary = root.with_extension("bin");
        fs::write(&input, source).expect("write fixture");
        let build = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["build", input.to_str().unwrap(), "--backend", backend, "--emit", "exe", "-o"])
            .arg(&binary)
            .output()
            .expect("start the build");
        assert!(
            build.status.success(),
            "{label} on {backend}: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&binary).status().expect("run the program");
        assert_eq!(status.code(), Some(42), "{label} on {backend}");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(binary);
        ran += 1;
    }
    ran
}

const PRELUDE: &str = "meta copy struct Stamp { tick: u32, } meta copy enum Level { Low, High, } \
struct Plain { value: Int, } ";

#[test]
fn check_accepts_a_copy_of_an_array_of_copy_types() {
    for element in ["Int", "u8", "Bool", "Stamp", "Level", "Array[u8, 2]"] {
        let body = format!(
            "erg xs: Array[{element}, 2] = Array[{element}, 2](); \
             erg ys: Array[{element}, 2] = copy(value: abs xs);"
        );
        let output = actus(
            &element.replace(['[', ']', ',', ' '], "-"),
            "check",
            &format!("{PRELUDE}verb main() {{ {body} }}"),
        );
        assert!(output.status.success(), "{element}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn check_rejects_a_copy_of_an_array_of_non_copy_types() {
    for element in ["Buffer", "Plain"] {
        let body = format!(
            "erg xs: Array[{element}, 2] = Array[{element}, 2](); \
             erg ys: Array[{element}, 2] = copy(value: abs xs);"
        );
        let output = actus(element, "check", &format!("{PRELUDE}verb main() {{ {body} }}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{element}: must be rejected");
        assert!(stderr.contains("E1021"), "{element}: {stderr}");
    }
}

#[test]
fn native_copy_of_an_array_has_storage_of_its_own() {
    let cases = [
        (
            "scalars",
            "erg xs: Array[Int, 3] = Array[Int, 3](); xs[0] = 5; \
             erg ys: Array[Int, 3] = copy(value: abs xs); ys[0] = 7; \
             return xs[0] * 10 + ys[0] - 15;",
        ),
        (
            "structs",
            "erg xs: Array[Stamp, 2] = Array[Stamp, 2](); xs[0] = Stamp { tick: 1u32, }; \
             erg ys: Array[Stamp, 2] = copy(value: abs xs); ys[0] = Stamp { tick: 9u32, }; \
             return (xs[0].tick as Int) * 10 + (ys[0].tick as Int) + 23;",
        ),
        (
            "nested",
            "erg xs: Array[Array[u8, 2], 2] = Array[Array[u8, 2], 2](); xs[1][0] = 4u8; \
             erg ys: Array[Array[u8, 2], 2] = copy(value: abs xs); ys[1][0] = 8u8; \
             return (xs[1][0] as Int) * 10 + (ys[1][0] as Int) - 6;",
        ),
    ];
    for (label, body) in cases {
        let ran = run_on_every_backend(label, &format!("{PRELUDE}verb main() -> Int {{ {body} }}"));
        assert!(ran >= 1, "{label}: no back end is built in");
    }
}

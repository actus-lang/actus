//! A value moved before the only `break` of a loop is moved after the loop
//! (ADR-0016, ADR-0027), so the scope end must not clean it up a second time.
//! The program runs on every back end that is built in and, when `valgrind`
//! is installed, under valgrind, which reports a double free or a leak.
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

/// Builds `source` on every back end, runs it and returns how many back ends
/// ran. Each run must print `expected` and exit with 3, and, when `valgrind`
/// is installed, run clean under it (a block that stdio keeps reachable is not a leak).
fn run_on_every_backend(label: &str, source: &str, expected: &str) -> usize {
    let valgrind = Command::new("valgrind")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    let mut ran = 0;
    for backend in [LLVM, CRANELIFT].into_iter().flatten() {
        let root = std::env::temp_dir()
            .join(format!("actus-loop-breaks-{label}-{backend}-{}", std::process::id()));
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
        let output = Command::new(&binary).output().expect("run the program");
        assert_eq!(output.status.code(), Some(3), "{label} on {backend}");
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected, "{label} on {backend}");
        if valgrind {
            let checked = Command::new("valgrind")
                .args([
                    "--error-exitcode=99",
                    "--leak-check=full",
                    "--errors-for-leak-kinds=definite,indirect,possible",
                ])
                .arg(&binary)
                .output()
                .expect("start valgrind");
            assert_eq!(checked.status.code(), Some(3), "{label} on {backend}: valgrind");
        }
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(binary);
        ran += 1;
    }
    ran
}

const PRELUDE: &str = "struct Tracker { id: Int, } \
perform Drop for Tracker { verb drop(ins self: Tracker) { print(self.id); } } \
verb eat(dat packet: Tracker) { drop(packet); } ";

#[test]
fn a_value_moved_on_the_only_break_path_is_cleaned_up_once() {
    for (label, body) in [
        (
            "loop",
            "erg x = Tracker { id: 1, }; erg n: Int = 0; \
loop { n += 1; if n > 2 { eat(packet: x); break; } } print(9); return n;",
        ),
        (
            "nested",
            "erg x = Tracker { id: 1, }; erg n: Int = 0; \
loop { loop { n += 1; if n > 2 { eat(packet: x); break; } } break; } print(9); return n;",
        ),
        (
            "every break moves",
            "erg x = Tracker { id: 1, }; erg n: Int = 0; \
loop { n += 1; if n > 5 { eat(packet: x); break; } if n > 2 { eat(packet: x); break; } } print(9); return n;",
        ),
    ] {
        let source = format!("{PRELUDE}verb main() -> Int {{ {body} }}");
        assert_eq!(run_on_every_backend(label, &source, "1\n9\n"), BACKENDS_BUILT);
    }
}

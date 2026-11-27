//! Builds each cleanup program on every back end that is built in, runs it
//! plain and under valgrind, and compares its output and exit code.

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

pub(crate) const BACKENDS_BUILT: usize =
    cfg!(feature = "llvm") as usize + cfg!(feature = "cranelift") as usize;

/// Builds `source` on every back end and runs it plain and under valgrind.
/// Returns how many back ends ran and one message per back end whose output,
/// exit code or memory behaviour differs from the expectation, so that a single
/// failing test shows every back end that is wrong.
pub(crate) fn run_on_every_backend(label: &str, source: &str, expected: &str, code: i32) -> usize {
    let valgrind = Command::new("valgrind")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !valgrind {
        eprintln!("NOT RUN: valgrind is not installed ({label})");
    }
    let mut ran = 0;
    let mut failures = Vec::new();
    for backend in [LLVM, CRANELIFT].into_iter().flatten() {
        let root = std::env::temp_dir()
            .join(format!("actus-struct-cleanup-{label}-{backend}-{}", std::process::id()));
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.code() != Some(code) || stdout != expected {
            failures.push(format!(
                "{label} on {backend}: expected {expected:?} with exit {code}, got {stdout:?} with exit {:?}",
                output.status.code()
            ));
        }
        if valgrind {
            let checked = Command::new("valgrind")
                .args([
                    "--error-exitcode=99",
                    "--leak-check=full",
                    "--errors-for-leak-kinds=definite",
                ])
                .arg(&binary)
                .output()
                .expect("start valgrind");
            if checked.status.code() != Some(code) {
                failures.push(format!(
                    "{label} on {backend}: valgrind exit {:?}, expected {code}\n{}",
                    checked.status.code(),
                    String::from_utf8_lossy(&checked.stderr)
                        .lines()
                        .filter(|line| line.contains("lost") || line.contains("Invalid"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        }
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(binary);
        ran += 1;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    ran
}

/// `Tracker` prints its id when it is cleaned up. `Pair` owns two trackers,
/// `Cfg` two buffers, `Holder` one buffer. `Guarded` has its own `Drop`
/// contract and also owns a buffer; `Duo` has its own `Drop` contract, which
/// prints `0`, and owns two trackers. `Nest` owns a struct that owns a buffer
/// and a second buffer, `Deep` owns a pair and a tracker, `Shelf` an array of
/// buffers. `Msg` and `Slot` are
/// enums with a resource in one variant; `Mark` is an enum whose own `Drop`
/// prints `4`. The helper verbs take their argument
/// as `dat` and either ignore it, drop it, or count it, so the callee owns it.
pub(crate) const PRELUDE: &str = "struct Tracker { id: Int, } \
perform Drop for Tracker { verb drop(ins self: Tracker) { print(self.id); } } \
struct Pair { erg a: Tracker, erg b: Tracker, } \
struct Cfg { erg a: Buffer, erg b: Buffer, } \
struct Holder { erg buf: Buffer, } \
struct Guarded { erg buf: Buffer, id: Int, } \
perform Drop for Guarded { verb drop(ins self: Guarded) { print(self.id); } } \
struct Inner { erg buf: Buffer, } struct Nest { erg inner: Inner, erg z: Buffer, } \
struct Deep { erg p: Pair, erg t: Tracker, } \
enum Msg { Data(Buffer), Empty, } \
verb ignore_cfg(dat c: Cfg) {} verb ignore_guarded(dat g: Guarded) {} \
verb ignore_pair(dat p: Pair) {} verb ignore_nest(dat n: Nest) {} \
verb drop_holder(dat o: Holder) { drop(o); } verb drop_pair(dat p: Pair) { drop(p); } \
verb drop_guarded(dat g: Guarded) { drop(g); } \
struct Duo { erg a: Tracker, erg b: Tracker, } struct Shelf { erg slots: Array[Buffer, 2], } \
perform Drop for Duo { verb drop(ins self: Duo) { print(0); } } \
enum Mark { On, Off, } perform Drop for Mark { verb drop(ins self: Mark) { print(4); } } \
enum Slot { Held(Mark), Free, } \
verb count_pair(dat p: Pair) -> Int { return 5; } verb sink_tracker(dat t: Tracker) {} \
verb keep_slot(dat s: Slot) {} verb grow(dat b: Buffer) -> Buffer { return b; } ";

pub(crate) const PAIR: &str =
    "erg p: Pair = Pair { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }; ";
pub(crate) const CFG: &str = "erg c: Cfg = Cfg { a: Buffer[1], b: Buffer[1], }; ";
pub(crate) const HOLDER: &str = "erg h: Holder = Holder { buf: Buffer[1], }; ";
pub(crate) const GUARDED: &str = "erg g: Guarded = Guarded { buf: Buffer[1], id: 7, }; ";
pub(crate) const NEST: &str =
    "erg n: Nest = Nest { inner: Inner { buf: Buffer[1], }, z: Buffer[1], }; ";
pub(crate) const DUO: &str = "erg u: Duo = Duo { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }; ";
pub(crate) const DEEP: &str = "erg d: Deep = Deep { p: Pair { a: Tracker { id: 1, }, b: Tracker { id: 2, }, }, \
t: Tracker { id: 3, }, }; ";

/// Runs `body` as the body of `main` (which returns 5 unless the body returns
/// earlier) and checks the output and the memory behaviour on every back end.
pub(crate) fn check(label: &str, body: &str, expected: &str) {
    let source = format!("{PRELUDE}verb main() -> Int {{ {body} return 5; }}");
    assert_eq!(run_on_every_backend(label, &source, expected, 5), BACKENDS_BUILT);
}

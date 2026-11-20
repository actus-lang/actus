//! Every optimization level keeps the observable behavior of a program
//! (ADR-0114): each fixture in `tests/fixtures/optimization` is built with
//! the `debug`, `release` and `size` profiles on every compiled backend and
//! must exit with the same status, and every guard must still trap.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const PROFILES: [&str; 3] = ["debug", "release", "size"];

#[derive(Clone, Copy, Debug)]
enum Expected {
    Exit(i32),
    Trap,
}

fn backends() -> Vec<&'static str> {
    let mut backends = Vec::new();
    if cfg!(feature = "llvm") {
        backends.push("llvm");
    }
    if cfg!(feature = "cranelift") {
        backends.push("cranelift");
    }
    backends
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/optimization/{name}.act"))
}

fn work_directory(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-optimization-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create optimization fixture directory");
    root
}

fn build(source: &Path, profile: &str, backend: &str, emit: &str, output: &Path) {
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("build")
        .arg(source)
        .args(["--profile", profile, "--backend", backend, "--emit", emit, "-o"])
        .arg(output)
        .output()
        .expect("run actus build");
    assert!(
        build.status.success(),
        "{backend} {profile} build of {} failed: {}",
        source.display(),
        String::from_utf8_lossy(&build.stderr)
    );
}

/// A guard ends the program through the trap instruction: `SIGILL` from
/// `ud2` on x86, `SIGTRAP` from `brk` on AArch64. Any other signal, such as
/// a segmentation fault, means the guard did not run.
#[cfg(unix)]
fn trapped(status: ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    matches!(status.signal(), Some(4 | 5))
}

#[cfg(not(unix))]
fn trapped(status: ExitStatus) -> bool {
    !status.success() && status.code().is_some_and(|code| code < 0)
}

fn assert_matrix(name: &str, expected: Expected) {
    let source = fixture(name);
    let root = work_directory(name);
    for backend in backends() {
        for profile in PROFILES {
            let program = root.join(format!("{backend}-{profile}{}", std::env::consts::EXE_SUFFIX));
            build(&source, profile, backend, "exe", &program);
            let status = Command::new(&program).status().expect("run optimization fixture");
            match expected {
                Expected::Exit(code) => assert_eq!(
                    status.code(),
                    Some(code),
                    "{name}: {backend} {profile} exited with {status:?}"
                ),
                Expected::Trap => {
                    assert!(trapped(status), "{name}: {backend} {profile} did not trap: {status:?}")
                }
            }
        }
    }
    fs::remove_dir_all(root).expect("remove optimization fixture directory");
}

#[test]
fn collatz_is_identical_at_every_level() {
    assert_matrix("collatz", Expected::Exit(108));
}

#[test]
fn recursive_fibonacci_is_identical_at_every_level() {
    assert_matrix("fib", Expected::Exit(32));
}

#[test]
fn sieve_is_identical_at_every_level() {
    assert_matrix("sieve", Expected::Exit(205));
}

#[test]
fn wrapping_xorshift_is_identical_at_every_level() {
    assert_matrix("xorshift", Expected::Exit(9));
}

#[test]
fn struct_temporaries_in_a_loop_are_identical_at_every_level() {
    assert_matrix("struct_loop", Expected::Exit(97));
}

#[test]
fn division_by_zero_traps_at_every_level() {
    assert_matrix("division_by_zero", Expected::Trap);
}

#[test]
fn remainder_by_zero_traps_at_every_level() {
    assert_matrix("remainder_by_zero", Expected::Trap);
}

#[test]
fn signed_division_overflow_traps_at_every_level() {
    assert_matrix("signed_division_overflow", Expected::Trap);
}

#[test]
fn shift_count_out_of_range_traps_at_every_level() {
    assert_matrix("shift_count", Expected::Trap);
}

#[test]
fn array_index_out_of_bounds_traps_at_every_level() {
    assert_matrix("array_index", Expected::Trap);
}

#[test]
fn checked_cast_overflow_traps_at_every_level() {
    assert_matrix("cast_overflow", Expected::Trap);
}

/// ADR-0115: `+`, `-`, `*`, unary `-` and `+=` trap on overflow at every
/// level; results at a bound and the wrapping and saturating operators do not.
#[test]
fn addition_overflow_traps_at_every_level() {
    assert_matrix("add_overflow", Expected::Trap);
}

#[test]
fn subtraction_underflow_traps_at_every_level() {
    assert_matrix("subtract_underflow", Expected::Trap);
}

#[test]
fn multiplication_overflow_traps_at_every_level() {
    assert_matrix("multiply_overflow", Expected::Trap);
}

#[test]
fn negation_overflow_traps_at_every_level() {
    assert_matrix("negate_overflow", Expected::Trap);
}

#[test]
fn compound_assignment_overflow_traps_at_every_level() {
    assert_matrix("compound_overflow", Expected::Trap);
}

#[test]
fn bounds_wrapping_and_saturation_are_identical_at_every_level() {
    assert_matrix("arithmetic_bounds", Expected::Exit(7));
}

/// The same source and profile give byte-identical objects, and the
/// `release` and `size` pipelines change the object of an unoptimized build.
#[cfg(feature = "llvm")]
#[test]
fn optimized_llvm_objects_are_deterministic() {
    let source = fixture("collatz");
    let root = work_directory("determinism");
    let objects = PROFILES.map(|profile| {
        let [first, second] = ["first", "second"].map(|run| {
            let path = root.join(format!("{profile}-{run}.o"));
            build(&source, profile, "llvm", "obj", &path);
            fs::read(path).expect("read optimization object")
        });
        assert_eq!(first, second, "{profile} objects differ between two builds");
        first
    });
    assert_ne!(objects[0], objects[1], "release object equals the debug object");
    assert_ne!(objects[0], objects[2], "size object equals the debug object");
    fs::remove_dir_all(root).expect("remove determinism directory");
}

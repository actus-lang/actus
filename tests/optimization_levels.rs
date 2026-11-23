//! Every optimization level keeps the observable behavior of a program
//! (ADR-0114): each fixture in `tests/fixtures/optimization` is built with
//! the `debug`, `release` and `size` profiles on every compiled backend and
//! must exit with the same status, and every failed guard must still be
//! reported (ADR-0117).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PROFILES: [&str; 3] = ["debug", "release", "size"];

#[derive(Clone, Copy, Debug)]
enum Expected {
    Exit(i32),
    /// The program stops with status 101 and reports this failed check.
    CheckFailure(&'static str),
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

fn assert_matrix(name: &str, expected: Expected) {
    let source = fixture(name);
    let root = work_directory(name);
    for backend in backends() {
        for profile in PROFILES {
            let program = root.join(format!("{backend}-{profile}{}", std::env::consts::EXE_SUFFIX));
            build(&source, profile, backend, "exe", &program);
            let execution = Command::new(&program).output().expect("run optimization fixture");
            let status = execution.status;
            match expected {
                Expected::Exit(code) => assert_eq!(
                    status.code(),
                    Some(code),
                    "{name}: {backend} {profile} exited with {status:?}"
                ),
                Expected::CheckFailure(message) => {
                    let stderr = String::from_utf8_lossy(&execution.stderr);
                    assert_eq!(status.code(), Some(101), "{name}: {backend} {profile}: {status:?}");
                    assert!(
                        stderr.starts_with(&format!("error: {message} at "))
                            && stderr.contains(&format!("{name}.act:")),
                        "{name}: {backend} {profile} reported {stderr:?}"
                    );
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
fn division_by_zero_is_reported_at_every_level() {
    assert_matrix("division_by_zero", Expected::CheckFailure("division by zero"));
}

#[test]
fn remainder_by_zero_is_reported_at_every_level() {
    assert_matrix("remainder_by_zero", Expected::CheckFailure("division by zero"));
}

#[test]
fn signed_division_overflow_is_reported_at_every_level() {
    assert_matrix("signed_division_overflow", Expected::CheckFailure("integer overflow"));
}

#[test]
fn shift_count_out_of_range_is_reported_at_every_level() {
    assert_matrix("shift_count", Expected::CheckFailure("shift count out of range"));
}

#[test]
fn array_index_out_of_bounds_is_reported_at_every_level() {
    assert_matrix("array_index", Expected::CheckFailure("index out of bounds"));
}

#[test]
fn checked_cast_overflow_is_reported_at_every_level() {
    assert_matrix("cast_overflow", Expected::CheckFailure("integer value out of range"));
}

/// ADR-0115: `+`, `-`, `*`, unary `-` and `+=` trap on overflow at every
/// level; results at a bound and the wrapping and saturating operators do not.
#[test]
fn addition_overflow_is_reported_at_every_level() {
    assert_matrix("add_overflow", Expected::CheckFailure("integer overflow"));
}

#[test]
fn subtraction_underflow_is_reported_at_every_level() {
    assert_matrix("subtract_underflow", Expected::CheckFailure("integer overflow"));
}

#[test]
fn multiplication_overflow_is_reported_at_every_level() {
    assert_matrix("multiply_overflow", Expected::CheckFailure("integer overflow"));
}

#[test]
fn negation_overflow_is_reported_at_every_level() {
    assert_matrix("negate_overflow", Expected::CheckFailure("integer overflow"));
}

#[test]
fn compound_assignment_overflow_is_reported_at_every_level() {
    assert_matrix("compound_overflow", Expected::CheckFailure("integer overflow"));
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

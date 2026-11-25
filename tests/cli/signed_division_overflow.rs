//! Signed `MIN / -1` stops the program with `integer overflow` and its
//! source location, and `MIN % -1` is `0`, for constant and non-constant
//! operands in every integer width, on every backend and optimization
//! profile (ADR-0115, ADR-0117). Constant operands are folded by the
//! optimizer, so each case is built with the `debug` and `release` profiles.

use super::*;

/// The native backends this test build can emit with.
fn backends() -> Vec<&'static str> {
    [cfg!(feature = "llvm").then_some("llvm"), cfg!(feature = "cranelift").then_some("cranelift")]
        .into_iter()
        .flatten()
        .collect()
}

/// The widths under test with their minimum value: machine widths and one
/// width that is not a machine width.
const TYPES: [(&str, &str); 4] = [
    ("i8", "-128"),
    ("i32", "-2147483648"),
    ("i64", "-9223372036854775808"),
    ("i40", "-549755813888"),
];

/// Builds `source` with `backend` and `profile`, runs it, and returns the
/// exit status, standard output and standard error.
fn run(source: &str, backend: &str, profile: &str) -> (Option<i32>, String, String) {
    // Tests of this file run in parallel in one process: every call needs a
    // directory of its own.
    static NEXT_DIRECTORY: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let unique = NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join(format!("actus-signed-division-{backend}-{profile}-{}-{unique}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create test directory");
    fs::write(root.join("main.act"), source).expect("write source");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .current_dir(&root)
        .args(["build", "main.act", "--emit", "exe", "--backend", backend, "--profile", profile])
        .args(["-o", "main.out"])
        .output()
        .expect("run actus build");
    assert!(
        build.status.success(),
        "{backend} {profile} build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let execution = Command::new(root.join("main.out")).output().expect("run the built program");
    let _ = fs::remove_dir_all(root);
    (
        execution.status.code(),
        String::from_utf8_lossy(&execution.stdout).into_owned(),
        String::from_utf8_lossy(&execution.stderr).into_owned(),
    )
}

/// `minimum <operator> -1` with constant operands, on one line of `main`.
fn constant_source(ty: &str, minimum: &str, operator: &str) -> String {
    format!(
        "verb main() {{\n    erg m: {ty} = {minimum}{ty};\n    erg d = m {operator} -1{ty};\n    print(int: d as i64);\n}}\n"
    )
}

/// `minimum <operator> -1` through a verb whose parameters are not constants.
fn dynamic_source(ty: &str, minimum: &str, operator: &str) -> String {
    format!(
        "verb apply(erg m: {ty}, erg n: {ty}) -> {ty} {{\n    return m {operator} n;\n}}\nverb main() {{\n    erg a: {ty} = {minimum}{ty};\n    erg b: {ty} = -1{ty};\n    erg d = apply(m: a, n: b);\n    print(int: d as i64);\n}}\n"
    )
}

#[test]
fn constant_minimum_divided_by_minus_one_reports_overflow() {
    for (ty, minimum) in TYPES {
        for backend in backends() {
            for profile in ["debug", "release"] {
                let (code, stdout, stderr) =
                    run(&constant_source(ty, minimum, "/"), backend, profile);
                let case = format!("{ty} constant with {backend} {profile}");
                assert_eq!(code, Some(101), "{case}");
                assert_eq!(stdout, "", "{case}");
                assert_eq!(stderr, "error: integer overflow at main.act:3:13\n", "{case}");
            }
        }
    }
}

#[test]
fn non_constant_minimum_divided_by_minus_one_reports_overflow() {
    for (ty, minimum) in TYPES {
        for backend in backends() {
            for profile in ["debug", "release"] {
                let (code, stdout, stderr) =
                    run(&dynamic_source(ty, minimum, "/"), backend, profile);
                let case = format!("{ty} non-constant with {backend} {profile}");
                assert_eq!(code, Some(101), "{case}");
                assert_eq!(stdout, "", "{case}");
                assert_eq!(stderr, "error: integer overflow at main.act:2:12\n", "{case}");
            }
        }
    }
}

#[test]
fn minimum_remainder_minus_one_is_zero_for_every_operand_kind() {
    for (ty, minimum) in TYPES {
        for backend in backends() {
            for profile in ["debug", "release"] {
                for source in [constant_source(ty, minimum, "%"), dynamic_source(ty, minimum, "%")]
                {
                    let (code, stdout, stderr) = run(&source, backend, profile);
                    let case = format!("{ty} remainder with {backend} {profile}: {source}");
                    assert_eq!(code, Some(0), "{case}");
                    assert_eq!(stdout, "0\n", "{case}");
                    assert_eq!(stderr, "", "{case}");
                }
            }
        }
    }
}

//! Prefix operators bind tighter than `as` (bug #35): `-128 as i8` is the valid `(-128) as i8`,
//! `-1 as u8` is rejected at compile time, `-x as i64` negates in the type of `x`, and the
//! explicit `-(1 as u8)` still traps at run time. Every program runs on every compiled backend.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn backends() -> Vec<&'static str> {
    let mut backends = Vec::new();
    if cfg!(feature = "cranelift") {
        backends.push("cranelift");
    }
    if cfg!(feature = "llvm") {
        backends.push("llvm");
    }
    backends
}

fn work_directory(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create work directory");
    root
}

/// Builds `source` for `backend`; returns the build's stderr on rejection or the program's exit
/// code and stderr after running it.
fn build_and_run(name: &str, source: &str, backend: &str) -> Result<(i32, String), String> {
    let root = work_directory(&format!("{name}-{backend}"));
    let input = root.join(format!("{name}.act"));
    let program = root.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    fs::write(&input, source).expect("write fixture");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("build")
        .arg(&input)
        .args(["--backend", backend, "--emit", "exe", "-o"])
        .arg(&program)
        .output()
        .expect("run actus build");
    if !build.status.success() {
        let _ = fs::remove_dir_all(&root);
        return Err(String::from_utf8_lossy(&build.stderr).into_owned());
    }
    let run = Command::new(&program).output().expect("run fixture");
    let _ = fs::remove_dir_all(&root);
    Ok((run.status.code().expect("exit code"), String::from_utf8_lossy(&run.stderr).into_owned()))
}

#[test]
fn a_negated_literal_cast_is_the_negative_value_on_every_backend() {
    let source = "verb main() -> Int {\n    erg low: i8 = -128 as i8;\n    erg wide: i8 = -128i32 as i8;\n    erg double: i8 = - -5 as i8;\n    if low != wide { return 1; }\n    return (low as Int) + (double as Int) + 200;\n}\n";
    for backend in backends() {
        // -128 + 5 + 200.
        let (code, _) = build_and_run("neg-literal-cast", source, backend).expect("must build");
        assert_eq!(code, 77, "{backend}");
    }
}

#[test]
fn a_negative_literal_cast_to_unsigned_is_rejected_before_code_generation() {
    let source = "verb main() -> Int {\n    erg a: u8 = -1 as u8;\n    return a as Int;\n}\n";
    for backend in backends() {
        let error = build_and_run("neg-unsigned-cast", source, backend)
            .expect_err("-1 as u8 must not build");
        assert!(error.contains("E1091"), "{backend}: {error}");
        assert!(error.contains("literal `-1` is outside the range of `u8`"), "{backend}: {error}");
    }
}

#[test]
fn negating_the_minimum_traps_in_the_binding_type_not_the_wider_type() {
    // Under the old `-(x as i64)` reading this printed 2147483648 and exited normally.
    let source = "verb widen(erg x: i32) -> i64 {\n    return -x as i64;\n}\n\nverb main() -> Int {\n    erg minimum: i32 = -2147483648i32;\n    erg a: i64 = widen(x: erg minimum);\n    return (a as Int);\n}\n";
    for backend in backends() {
        let (code, stderr) =
            build_and_run("neg-minimum-cast", source, backend).expect("must build");
        assert_eq!(code, 101, "{backend}: {stderr}");
        assert!(stderr.contains("integer overflow"), "{backend}: {stderr}");
    }
}

#[test]
fn an_explicitly_grouped_cast_is_negated_after_the_cast_and_traps() {
    let source = "verb main() -> Int {\n    erg one: u8 = 1;\n    erg a: u8 = -(one as u8);\n    return a as Int;\n}\n";
    for backend in backends() {
        let (code, stderr) =
            build_and_run("neg-grouped-cast", source, backend).expect("must build");
        assert_eq!(code, 101, "{backend}: {stderr}");
        assert!(stderr.contains("integer overflow"), "{backend}: {stderr}");
    }
}

#[test]
fn bitwise_and_wrapping_prefix_operators_apply_before_the_cast() {
    // `~b as u16` is `(~b) as u16` = 254; the old `~(b as u16)` was 65534.
    // `-%n as u8` is `(-%n) as u8` = 1; the old `-%(n as u8)` trapped on the cast of -1.
    let source = "verb main() -> Int {\n    erg b: u8 = 1;\n    erg n: i32 = -1;\n    erg complement: u16 = ~b as u16;\n    erg wrapped: u8 = -%n as u8;\n    if complement != 254u16 { return 1; }\n    if wrapped != 1u8 { return 2; }\n    return 42;\n}\n";
    for backend in backends() {
        let (code, stderr) =
            build_and_run("prefix-cast-families", source, backend).expect("must build");
        assert_eq!(code, 42, "{backend}: {stderr}");
    }
}

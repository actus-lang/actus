//! ADR-0119 end to end: mixed integer operands are rejected by the driver, a
//! literal operand takes the other operand's type in generated code, and the
//! standard library and the examples still pass the checker.
#![cfg(unix)]

use std::fs;
use std::path::PathBuf;

use actus::cli::run_with_args;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn arguments(values: &[&str]) -> std::vec::IntoIter<String> {
    values.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>().into_iter()
}

/// Builds `source` with `backend`, runs it, and returns its exit code, or
/// `None` when the build is rejected.
fn build_and_run(name: &str, source: &str, backend: &str) -> Option<i32> {
    let stem = std::env::temp_dir().join(format!("{name}-{backend}-{}", std::process::id()));
    let input = stem.with_extension("act");
    let output = stem.with_extension("bin");
    fs::write(&input, source).expect("write fixture");
    let code = run_with_args(arguments(&[
        "build",
        &input.display().to_string(),
        "--backend",
        backend,
        "--emit",
        "exe",
        "-o",
        &output.display().to_string(),
    ]));
    let status = (code == 0).then(|| {
        std::process::Command::new(&output).status().expect("run fixture").code().expect("exit")
    });
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    status
}

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

#[test]
fn the_driver_rejects_mixed_integer_operands() {
    let dir = std::env::temp_dir().join(format!("actus-mixed-operands-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create fixture directory");
    let cases = [
        "verb main() -> Int { erg a: u8 = 1u8; erg b: u32 = 2u32; erg c = a + b; return 0; }",
        "verb main() -> Int { erg a: u8 = 1u8; erg b = a + (4 as Int); return 0; }",
        "verb main() -> Int { erg a: u32 = 1u32; erg b: i32 = 2i32; erg c = a & b; return 0; }",
        "verb main() -> Int { erg a: u8 = 1u8; erg b: Int = 2; a +%= b; return 0; }",
    ];
    for (index, source) in cases.iter().enumerate() {
        let path = dir.join(format!("mixed{index}.act"));
        fs::write(&path, source).expect("write fixture");
        let path = path.display().to_string();
        assert_eq!(run_with_args(arguments(&["check", &path])), 1, "{source}");
        assert_eq!(run_with_args(arguments(&["check", "--strict", &path])), 1, "{source}");
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn literal_operands_take_the_other_operands_type_in_generated_code() {
    // `100 - a` and `a * 3 + 2` stay `u8`; the sums are exact, so they run.
    let exact = "verb main() -> Int {\n    erg a: u8 = 10u8;\n    erg left: u8 = 100 - a;\n    erg right: u8 = a * 3 + 2;\n    return (left as Int) + (right as Int);\n}\n";
    // `1 + a` is `u8` arithmetic, so 255 overflows and traps instead of
    // producing 256 in `Int`.
    let trapping = "verb main() -> Int {\n    erg a: u8 = 255u8;\n    erg b = 1 + a;\n    return b as Int;\n}\n";
    for backend in backends() {
        assert_eq!(build_and_run("actus-literal-exact", exact, backend), Some(122), "{backend}");
        assert_eq!(build_and_run("actus-literal-trap", trapping, backend), Some(101), "{backend}");
    }
}

#[test]
fn the_standard_library_modules_still_pass_the_strict_checker() {
    let mut checked = 0;
    for module in fs::read_dir(root().join("library/std/src")).expect("std source root") {
        let module = module.expect("std entry").path();
        if !module.is_dir() {
            continue;
        }
        for file in fs::read_dir(&module).expect("std module") {
            let file = file.expect("std file").path();
            if file.extension().is_some_and(|extension| extension == "act") {
                let path = file.display().to_string();
                assert_eq!(run_with_args(arguments(&["check", "--strict", &path])), 0, "{path}");
                checked += 1;
            }
        }
    }
    assert!(checked > 40, "expected the std modules to be found, checked {checked}");
}

#[test]
fn the_examples_still_pass_the_checker() {
    let mut checked = 0;
    for entry in fs::read_dir(root().join("examples")).expect("examples") {
        let path = entry.expect("example entry").path();
        let file = if path.is_dir() {
            ["src/main.act", "main.act"]
                .iter()
                .map(|name| path.join(name))
                .find(|file| file.exists())
        } else {
            path.extension().is_some_and(|extension| extension == "act").then_some(path)
        };
        if let Some(file) = file {
            let file = file.display().to_string();
            assert_eq!(run_with_args(arguments(&["check", &file])), 0, "{file}");
            checked += 1;
        }
    }
    assert!(checked > 20, "expected the examples to be found, checked {checked}");
}

#[test]
fn the_operator_surface_example_runs_with_explicit_conversions() {
    let source = fs::read_to_string(root().join("examples/operator_surface.act")).expect("example");
    for backend in backends() {
        assert_eq!(build_and_run("actus-operator-surface", &source, backend), Some(0), "{backend}");
    }
}

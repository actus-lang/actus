#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
#[test]
fn project_workflow_runs_console_application_through_std_io() {
    let root = std::env::temp_dir().join(format!("actus-console-project-{}", std::process::id()));
    let output = root.join("console");
    write_console_project(&root);

    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("check")
        .current_dir(&root)
        .output()
        .expect("run project check");
    assert!(check.status.success(), "stderr: {}", String::from_utf8_lossy(&check.stderr));

    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&root)
        .output()
        .expect("build console application");
    assert!(build.status.success(), "stderr: {}", String::from_utf8_lossy(&build.stderr));
    let built = Command::new(&output).output().expect("run built console application");
    assert_eq!(built.status.code(), Some(9));
    assert_eq!(built.stdout, b"OK\n");
    assert_eq!(built.stderr, b"OK\n");

    let run = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("run")
        .current_dir(&root)
        .output()
        .expect("run project console application");
    assert_eq!(run.status.code(), Some(9));
    assert_eq!(run.stdout, b"OK\n");
    assert!(String::from_utf8_lossy(&run.stderr).contains("process exited with status 9"));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
fn write_console_project(root: &Path) {
    let standard_library = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std");
    fs::create_dir_all(root.join("src")).expect("create project source directory");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"console\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[dependencies]\nio = {{ path = \"{}\" }}\n",
            standard_library.display()
        ),
    )
    .expect("write project manifest");
    fs::write(
        root.join("src/main.act"),
        "import { open io }; verb main() -> Int { erg text = Buffer[0]; append(text, 79); append(text, 75); printlnb(text: abs text); eprintln(buffer: abs text); return 9; }\n",
    )
    .expect("write console application");
}

/// Backends compiled into this test build.
#[cfg(unix)]
fn enabled_backends() -> Vec<&'static str> {
    let backends: Vec<&'static str> = [
        #[cfg(feature = "llvm")]
        "llvm",
        #[cfg(feature = "cranelift")]
        "cranelift",
    ]
    .to_vec();
    assert!(!backends.is_empty(), "no native backend is enabled");
    backends
}

/// Builds a single-file program with every enabled backend, runs it, and
/// returns each backend's name with its output.
#[cfg(unix)]
fn build_and_run_on_each_backend(
    name: &str,
    source: &str,
) -> Vec<(&'static str, std::process::Output)> {
    enabled_backends()
        .into_iter()
        .map(|backend| {
            let root = std::env::temp_dir()
                .join(format!("actus-console-{name}-{backend}-{}", std::process::id()));
            let input = root.with_extension("act");
            let output = root.with_extension("bin");
            fs::write(&input, source).expect("write console source");
            let build = Command::new(env!("CARGO_BIN_EXE_actus"))
                .args(["build", "--emit", "exe", "--backend", backend, "-o"])
                .arg(&output)
                .arg(&input)
                .output()
                .expect("build console source");
            assert!(
                build.status.success(),
                "{backend} build failed: {}",
                String::from_utf8_lossy(&build.stderr)
            );
            let run = Command::new(&output).output().expect("run console executable");
            let _ = fs::remove_file(input);
            let _ = fs::remove_file(output);
            (backend, run)
        })
        .collect()
}

#[cfg(unix)]
#[test]
fn built_in_eprint_writes_labelled_values_to_stderr() {
    let source = "verb main() { eprint(string: \"e\"); eprint(int: 5); eprint(bool: true); }\n";
    for (backend, run) in build_and_run_on_each_backend("eprint", source) {
        assert_eq!(run.stderr, b"e\n5\ntrue\n", "{backend} stderr");
        assert!(run.stdout.is_empty(), "{backend} stdout");
    }
}

#[cfg(unix)]
#[test]
fn built_in_eprint_writes_a_borrowed_string_to_stderr() {
    let source = "verb main() { erg text: String = \"e\"; eprint(string: abs text); }\n";
    for (backend, run) in build_and_run_on_each_backend("eprint-abs", source) {
        assert_eq!(run.stderr, b"e\n", "{backend} stderr");
        assert!(run.stdout.is_empty(), "{backend} stdout");
    }
}

#[cfg(unix)]
#[test]
fn built_in_print_of_a_byte_writes_its_decimal_value() {
    let source = "verb main() { erg b: u8 = 200u8; print(byte: 7u8); print(byte: b); }\n";
    for (backend, run) in build_and_run_on_each_backend("print-byte", source) {
        assert_eq!(run.stdout, b"7\n200\n", "{backend} stdout");
        assert!(run.stderr.is_empty(), "{backend} stderr");
    }
}

#[cfg(unix)]
#[test]
fn built_in_eprint_of_a_byte_writes_its_decimal_value_to_stderr() {
    let source = "verb main() { erg b: u8 = 200u8; eprint(byte: b); }\n";
    for (backend, run) in build_and_run_on_each_backend("eprint-byte", source) {
        assert_eq!(run.stderr, b"200\n", "{backend} stderr");
        assert!(run.stdout.is_empty(), "{backend} stdout");
    }
}

#[cfg(unix)]
#[test]
fn negated_literals_at_the_type_minimum_run_as_single_constants() {
    let source = "verb main() { erg a: i8 = -128i8; print(int: a as Int); erg m: i32 = -2147483648i32; print(int: m as Int); erg n = -2147483648; print(int: n); erg w: i64 = -9223372036854775808i64; print(int: (w + 1i64) as Int); erg big: i64 = -3000000000; print(int: (big / 1000000i64) as Int); }\n";
    for (backend, run) in build_and_run_on_each_backend("negated-minimums", source) {
        assert_eq!(run.stdout, b"-128\n-2147483648\n-2147483648\n1\n-3000\n", "{backend} stdout");
        assert!(run.stderr.is_empty(), "{backend} stderr");
    }
}

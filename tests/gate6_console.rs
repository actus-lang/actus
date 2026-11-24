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

/// Every integer type with its minimum and maximum value as source text and as
/// the decimal text `print(int:)` must write for it.
#[cfg(unix)]
const INTEGER_EXTREMES: [(&str, &str, &str); 11] = [
    ("i8", "-128i8", "-128"),
    ("i8", "127i8", "127"),
    ("i16", "-32768i16", "-32768"),
    ("i16", "32767i16", "32767"),
    ("i32", "-2147483648i32", "-2147483648"),
    ("i32", "2147483647i32", "2147483647"),
    ("i64", "-9223372036854775808i64", "-9223372036854775808"),
    ("i64", "9223372036854775807i64", "9223372036854775807"),
    ("u8", "255u8", "255"),
    ("u16", "65535u16", "65535"),
    ("u32", "4294967295u32", "4294967295"),
];

/// A program printing every extreme value both through a typed variable and
/// as a literal with `verb_name`, plus `u64`, `Usize`, `Int` and odd widths.
#[cfg(unix)]
fn integer_print_program(verb_name: &str) -> (String, String) {
    let mut body = String::new();
    let mut expected = String::new();
    let mut extremes = INTEGER_EXTREMES.to_vec();
    extremes.extend([
        ("u64", "18446744073709551615u64", "18446744073709551615"),
        ("u64", "0u64", "0"),
        ("u24", "16777215u24", "16777215"),
        ("i40", "-5i40", "-5"),
        ("Usize", "18446744073709551615 as Usize", "18446744073709551615"),
        ("Int", "-2147483648", "-2147483648"),
        ("Int", "2147483647", "2147483647"),
    ]);
    for (index, (type_name, literal, text)) in extremes.iter().enumerate() {
        body.push_str(&format!(
            "erg v{index}: {type_name} = {literal}; {verb_name}(int: v{index}); "
        ));
        expected.push_str(&format!("{text}\n"));
    }
    for (type_name, literal, text) in &extremes {
        if *type_name != "Usize" && *type_name != "Int" && *type_name != "i40" {
            body.push_str(&format!("{verb_name}(int: {literal}); "));
            expected.push_str(&format!("{text}\n"));
        }
    }
    (format!("verb main() {{ {body}}}\n"), expected)
}

#[cfg(unix)]
#[test]
fn built_in_print_of_an_integer_writes_the_exact_value_of_every_type() {
    let (source, expected) = integer_print_program("print");
    for (backend, run) in build_and_run_on_each_backend("print-integers", &source) {
        assert_eq!(String::from_utf8_lossy(&run.stdout), expected, "{backend} stdout");
        assert!(run.stderr.is_empty(), "{backend} stderr");
    }
}

#[cfg(unix)]
#[test]
fn built_in_eprint_of_an_integer_writes_the_exact_value_of_every_type() {
    let (source, expected) = integer_print_program("eprint");
    for (backend, run) in build_and_run_on_each_backend("eprint-integers", &source) {
        assert_eq!(String::from_utf8_lossy(&run.stderr), expected, "{backend} stderr");
        assert!(run.stdout.is_empty(), "{backend} stdout");
    }
}

#[cfg(unix)]
#[test]
fn built_in_print_of_an_integer_expression_uses_the_type_of_the_expression() {
    let source = "verb main() { erg a: u64 = 18446744073709551615u64; print(int: a - 1u64); erg w: i64 = -9223372036854775807i64; print(int: w - 1i64); print(int: 4000000000u32 / 2u32); erg b: u8 = 200u8; print(int: b as u64 * 100000000000u64); }\n";
    for (backend, run) in build_and_run_on_each_backend("print-integer-expressions", source) {
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            "18446744073709551614\n-9223372036854775808\n2000000000\n20000000000000\n",
            "{backend} stdout"
        );
        assert!(run.stderr.is_empty(), "{backend} stderr");
    }
}

#[cfg(unix)]
#[test]
fn built_in_print_of_an_integer_wider_than_64_bits_is_a_compile_error() {
    for (verb_name, source) in [
        ("print", "verb main() { erg a: u128 = 5u128; print(int: a); }\n"),
        ("eprint", "verb main() { eprint(int: 5i128); }\n"),
    ] {
        let input = std::env::temp_dir()
            .join(format!("actus-console-wide-{verb_name}-{}.act", std::process::id()));
        fs::write(&input, source).expect("write wide integer source");
        let check = Command::new(env!("CARGO_BIN_EXE_actus"))
            .arg("check")
            .arg(&input)
            .output()
            .expect("run check");
        let _ = fs::remove_file(&input);
        let stderr = String::from_utf8_lossy(&check.stderr);
        assert!(!check.status.success(), "{verb_name} accepted a wide integer");
        assert!(stderr.contains("E1139"), "{verb_name}: {stderr}");
        assert!(stderr.contains("128"), "{verb_name} must name the type: {stderr}");
    }
}

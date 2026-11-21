//! A failed runtime check in a hosted executable reports the check and its
//! source location and exits with status 101 (ADR-0117).

use super::*;

/// The native backends this test build can emit with.
fn backends() -> Vec<&'static str> {
    [cfg!(feature = "llvm").then_some("llvm"), cfg!(feature = "cranelift").then_some("cranelift")]
        .into_iter()
        .flatten()
        .collect()
}

/// Builds `source` as `main.act` with `backend` and `emit`, from inside a
/// fresh directory so reported paths are relative to it.
fn build(name: &str, source: &str, backend: &str, emit: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir()
        .join(format!("actus-runtime-check-{name}-{backend}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create test directory");
    fs::write(root.join("main.act"), source).expect("write source");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .current_dir(&root)
        .args(["build", "main.act", "--emit", emit, "--backend", backend, "-o", "main.out"])
        .output()
        .expect("run actus build");
    assert!(
        build.status.success(),
        "`{name}` must build with {backend}: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    root
}

/// Builds and runs `source` with every backend and checks that it stops
/// with `message` after writing `stdout`.
fn assert_check_fails(name: &str, source: &str, stdout: &str, message: &str) {
    for backend in backends() {
        let root = build(name, source, backend, "exe");
        let execution =
            Command::new(root.join("main.out")).output().expect("run the built program");
        assert_eq!(execution.status.code(), Some(101), "{name} with {backend}");
        assert_eq!(String::from_utf8_lossy(&execution.stdout), stdout, "{name} with {backend}");
        assert_eq!(
            String::from_utf8_lossy(&execution.stderr),
            format!("error: {message}\n"),
            "{name} with {backend}"
        );
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn failed_overflow_check_reports_its_location() {
    assert_check_fails(
        "overflow",
        "verb main() {\n    erg x: u8 = 250u8;\n    erg y: u8 = x + 10u8;\n    print(int: y as Int);\n}\n",
        "",
        "integer overflow at main.act:3:17",
    );
}

#[test]
fn failed_division_check_reports_its_location() {
    assert_check_fails(
        "division",
        "verb main() {\n    erg zero = 0;\n    erg quotient = 10 / zero;\n    print(int: quotient);\n}\n",
        "",
        "division by zero at main.act:3:20",
    );
}

#[test]
fn failed_index_check_reports_its_location() {
    assert_check_fails(
        "index",
        "verb main() {\n    erg values: Array[Int, 4] = Array[Int, 4]();\n    erg index: Usize = 7;\n    print(int: values[index]);\n}\n",
        "",
        "index out of bounds at main.act:4:16",
    );
}

#[test]
fn failed_arena_check_reports_its_location() {
    assert_check_fails(
        "arena",
        "struct Node {\n    value: Int,\n}\n\nverb main() -> Int {\n    erg arena: Arena[4] = Arena[4]();\n    erg first: Node = arena.place(value: Node { value: 1, });\n    erg second: Node = arena.place(value: Node { value: 2, });\n    return first.value + second.value;\n}\n",
        "",
        "arena capacity exceeded at main.act:8:24",
    );
}

#[test]
fn output_written_before_a_failed_check_is_kept() {
    assert_check_fails(
        "output",
        "verb main() {\n    print(string: \"before\");\n    erg x: u8 = 255u8;\n    erg y: u8 = x + 1u8;\n    print(int: y as Int);\n}\n",
        "before\n",
        "integer overflow at main.act:4:17",
    );
}

#[test]
fn object_output_keeps_trapping_checks() {
    for backend in backends() {
        let root = build(
            "object",
            "verb main() -> Int {\n    erg x: u8 = 250u8;\n    erg y: u8 = x + 10u8;\n    return y as Int;\n}\n",
            backend,
            "obj",
        );
        let bytes = fs::read(root.join("main.out")).expect("read object");
        let object = object::File::parse(bytes.as_slice()).expect("parse object");
        let reports = object::Object::symbols(&object)
            .any(|symbol| object::ObjectSymbol::name(&symbol) == Ok("actus_runtime_check_failed"));
        assert!(!reports, "an object for {backend} must trap without the runtime");
        let _ = fs::remove_dir_all(root);
    }
}

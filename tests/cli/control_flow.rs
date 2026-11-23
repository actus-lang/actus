#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
use super::*;
#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
static NEXT_VALUE_IF_FIXTURE: AtomicUsize = AtomicUsize::new(0);

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
fn run_shared_value_if_fixture(backend: &str, source: &str) -> i32 {
    let fixture_id = NEXT_VALUE_IF_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join(format!("actus-cli-value-if-{backend}-{}-{fixture_id}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create value-if fixture directory");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"value-if-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
        ),
    )
    .expect("write value-if fixture manifest");
    fs::write(&input, source).expect("write value-if fixture source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run value-if fixture");
    let code = status.code().expect("value-if fixture should have an exit code");
    let _ = fs::remove_dir_all(root);
    code
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn cranelift_backend_builds_and_runs_shared_value_if_expression() {
    assert_eq!(
        run_shared_value_if_fixture(
            "cranelift",
            "verb main() -> Int { erg selected = if true { 41 } else { 1 }; return selected + 1; }\n",
        ),
        42
    );
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_shared_value_if_expression() {
    assert_eq!(
        run_shared_value_if_fixture(
            "llvm",
            "verb main() -> Int { erg selected = if true { 41 } else { 1 }; return selected + 1; }\n",
        ),
        42
    );
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn cranelift_backend_builds_and_runs_nested_shared_value_if_expression() {
    assert_eq!(
        run_shared_value_if_fixture(
            "cranelift",
            "verb main() -> Int { return 1 + (if true { 41 } else { 1 }); }\n",
        ),
        42
    );
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_nested_shared_value_if_expression() {
    assert_eq!(
        run_shared_value_if_fixture(
            "llvm",
            "verb main() -> Int { return 1 + (if true { 41 } else { 1 }); }\n",
        ),
        42
    );
}

/// Runs `source` with both backends and checks its exit code.
#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
fn assert_exits_with(source: &str, expected: i32) {
    for backend in ["cranelift", "llvm"] {
        assert_eq!(run_shared_value_if_fixture(backend, source), expected, "{backend}");
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn a_for_range_without_a_type_counts_with_int() {
    assert_exits_with(
        "verb main() -> Int { erg total = 0; for erg index in 1..5 { total += index; } return total; }",
        10,
    );
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn a_for_range_without_a_type_takes_the_type_of_its_start() {
    assert_exits_with(
        "verb main() -> Int { erg last: u8 = 0u8; for erg index in 250u8..253u8 { last = index; } return last as Int; }",
        252,
    );
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn a_typed_for_range_gives_plain_literal_bounds_its_type() {
    assert_exits_with(
        "verb main() -> Int { erg total: u16 = 0u16; for erg index: u16 in 1..4 { total += index; } return total as Int; }",
        6,
    );
    assert_exits_with(
        "verb main() -> Int { erg total = 0; for erg index: Int in 1..4 { total += index; } return total; }",
        6,
    );
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn an_else_if_value_chain_selects_the_first_matching_arm() {
    for (value, expected) in [(12, 4), (8, 3), (4, 2), (1, 1)] {
        assert_exits_with(
            &format!(
                "verb main() -> Int {{ erg v = {value}; erg size = if v > 10 {{ 4 }} else if v > 7 {{ 3 }} else if v > 3 {{ 2 }} else {{ 1 }}; return size; }}"
            ),
            expected,
        );
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn an_else_if_value_chain_runs_only_the_selected_arm() {
    assert_exits_with(
        "verb main() -> Int { erg y = 1; erg r = if y == 0 { 0 } else if y == 1 { 7 } else { 10 / (y - 1) }; return r; }",
        7,
    );
}

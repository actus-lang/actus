#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
use super::*;

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
fn run_shared_value_if_fixture(backend: &str, source: &str) -> i32 {
    let root =
        std::env::temp_dir().join(format!("actus-cli-value-if-{backend}-{}", std::process::id()));
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

//! Strict mode measures one set of files with one counting rule: every
//! production `.act` file under the package source root, whether or not
//! anything imports it, counting code lines only (ADR-0041). `check`, `build`
//! and `test` must agree on both.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MANIFEST: &str = "[package]\nname = \"sources\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n";
const ENTRY: &str = "import {\n    open helper,\n};\n\n\"\"\"Entry point returning the helper value as the exit status.\"\"\"\nverb main() -> Int {\n    return helper_value();\n}\n";
const FACADE: &str = "\"\"\"Re-export the helper operations.\"\"\"\nopen ops;\n";
const HELPER_VERB: &str = "\"\"\"Return the constant the entry point reports.\"\"\"\nopen verb helper_value() -> Int {\n    return 42;\n}\n";

/// A package with an entry file, a `helper` module whose `ops.act` holds
/// `ops_source`, and the optional extra production file `src/orphan.act`.
fn package(name: &str, ops_source: &str, orphan: Option<&str>) -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-sources-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/helper")).expect("create module directory");
    fs::write(root.join("Actus.toml"), MANIFEST).expect("write manifest");
    fs::write(root.join("src/main.act"), ENTRY).expect("write entry");
    fs::write(root.join("src/helper/helper.act"), FACADE).expect("write facade");
    fs::write(root.join("src/helper/ops.act"), ops_source).expect("write module");
    if let Some(orphan) = orphan {
        fs::write(root.join("src/orphan.act"), orphan).expect("write orphan");
    }
    root
}

fn actus(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("run actus")
}

/// Runs `check`, `build` and `test` in strict mode and returns each command
/// with its output.
fn strict_commands(root: &Path) -> Vec<(&'static str, Output)> {
    let object = root.join("out.o");
    let object = object.to_string_lossy().into_owned();
    vec![
        ("check", actus(root, &["check", "--strict"])),
        ("build", actus(root, &["build", "--strict", "-o", &object])),
        ("test", actus(root, &["test", "--strict"])),
    ]
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `count` documented-free constant lines, which are code, not documentation.
fn constants(count: usize) -> String {
    (0..count).map(|index| format!("open const VALUE_{index}: Int = {index};\n")).collect()
}

/// One block documentation comment of `count` physical lines.
fn long_documentation(count: usize) -> String {
    let body = (0..count).map(|index| format!("Documentation line {index}.\n")).collect::<String>();
    format!("\"\"\"\n{body}\"\"\"\n")
}

fn assert_all_accept(root: &Path) {
    for (command, output) in strict_commands(root) {
        assert!(output.status.success(), "{command} should pass: {}", text(&output));
    }
}

fn assert_all_reject(root: &Path, code: &str, needle: &str) {
    for (command, output) in strict_commands(root) {
        let report = text(&output);
        assert!(!output.status.success(), "{command} should fail: {report}");
        assert!(report.contains(code), "{command} should report {code}: {report}");
        assert!(report.contains(needle), "{command} should name `{needle}`: {report}");
    }
}

#[test]
fn a_module_over_the_limit_only_with_documentation_passes_check_build_and_test() {
    // 450 code lines and 600 documentation lines: over 500 physical lines,
    // under 500 code lines.
    let source = format!("{}{HELPER_VERB}{}", long_documentation(600), constants(450));
    let root = package("documented", &source, None);
    assert_all_accept(&root);
}

#[test]
fn a_module_with_more_than_500_code_lines_fails_check_build_and_test() {
    let source = format!("{HELPER_VERB}{}", constants(520));
    let root = package("oversized", &source, None);
    assert_all_reject(&root, "E1850", "ops.act");
}

#[test]
fn an_unimported_file_is_measured_by_check_build_and_test() {
    let orphan = constants(520);
    let root = package("orphan", HELPER_VERB, Some(&orphan));
    assert_all_reject(&root, "E1850", "orphan.act");
}

#[test]
fn an_oversized_module_is_measured_when_check_names_the_module_file() {
    let source = format!("{HELPER_VERB}{}", constants(520));
    let root = package("explicit", &source, None);
    let output = actus(&root, &["check", "--strict", "src/helper/ops.act"]);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("E1850"), "{}", text(&output));
}

#[test]
fn a_private_verb_without_a_docstring_fails_check_build_and_test() {
    let source = format!("{HELPER_VERB}verb hidden_helper() -> Int {{\n    return 1;\n}}\n");
    let root = package("private-verb", &source, None);
    assert_all_reject(&root, "E1840", "verb hidden_helper");
}

#[test]
fn an_unimported_file_with_an_undocumented_private_verb_fails() {
    let orphan = "verb forgotten() -> Int {\n    return 1;\n}\n";
    let root = package("orphan-verb", HELPER_VERB, Some(orphan));
    assert_all_reject(&root, "E1840", "verb forgotten");
}

#[test]
fn a_documented_private_verb_passes_and_test_files_need_no_documentation() {
    let source = format!(
        "{HELPER_VERB}\"\"\"Return one; it exists to show a documented private verb.\"\"\"\nverb hidden_helper() -> Int {{\n    return 1;\n}}\n"
    );
    let root = package("documented-private", &source, None);
    fs::create_dir_all(root.join("tests")).expect("create tests directory");
    fs::write(root.join("tests/a.act"), "meta test\nverb passes() -> Int {\n    return 0;\n}\n")
        .expect("write test file");
    assert_all_accept(&root);
}

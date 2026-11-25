//! ADR-0121: module tests live in `tests` directories next to their module,
//! project tests in `tests/`, and every other place is a layout error.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A project with a `math` module (private verb `private_value`, open verb
/// `public_value`) and a `math::inner` submodule (open `inner_open`, private
/// `inner_private`). `files` are written relative to the project root.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-layout-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let base = [
        ("Actus.toml", "[package]\nname = \"layout\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n"),
        (
            "src/main.act",
            "import { open math };\nverb main() -> Int { return public_value() - 1; }\n",
        ),
        ("src/math/math.act", "open api;\nopen inner;\n"),
        (
            "src/math/api.act",
            "open verb public_value() -> Int { return 1; }\nverb private_value() -> Int { return 42; }\n",
        ),
        ("src/math/inner/inner.act", "open items;\n"),
        (
            "src/math/inner/items.act",
            "open verb inner_open() -> Int { return 5; }\nverb inner_private() -> Int { return 6; }\n",
        ),
    ];
    for (path, source) in base.iter().chain(files) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
        fs::write(path, source).expect("write fixture file");
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

fn text(output: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Runs `actus test`, which must pass, and returns its stdout.
fn passing(root: &Path, arguments: &[&str]) -> String {
    let output = actus(root, arguments);
    let (stdout, stderr) = text(&output);
    assert!(output.status.success(), "stdout: {stdout}\nstderr: {stderr}");
    stdout
}

/// Runs a command that must fail and returns its stderr.
fn failing(root: &Path, arguments: &[&str]) -> String {
    let output = actus(root, arguments);
    let (stdout, stderr) = text(&output);
    assert!(!output.status.success(), "unexpected success; stdout: {stdout}");
    stderr
}

#[test]
fn module_tests_are_discovered_and_named_by_their_module_path() {
    let root = project(
        "discovery",
        &[
            ("src/math/tests/first.act", "meta test\nverb accepts_header() -> Int { return 0; }\n"),
            ("src/ie/ie.act", "open persistence;\n"),
            ("src/ie/persistence/persistence.act", "open store;\n"),
            ("src/ie/persistence/store.act", "open verb stored() -> Int { return 3; }\n"),
            (
                "src/ie/persistence/tests/header.act",
                "meta test\nverb accepts_header() -> Int { return stored() - 3; }\n",
            ),
        ],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("test math::accepts_header ... ok"), "stdout: {stdout}");
    assert!(stdout.contains("test ie::persistence::accepts_header ... ok"), "stdout: {stdout}");
    assert!(stdout.contains("2 passed; 0 failed"), "stdout: {stdout}");
}

#[test]
fn project_tests_are_discovered_and_named_tests() {
    let root = project(
        "project-discovery",
        &[("tests/api.act", "meta test\nverb accepts_header() -> Int { return 0; }\n")],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("test tests::accepts_header ... ok"), "stdout: {stdout}");
    assert!(stdout.contains("1 passed; 0 failed"), "stdout: {stdout}");
}

#[test]
fn a_module_test_sees_the_private_verbs_of_its_module() {
    let root = project(
        "module-private",
        &[(
            "src/math/tests/private.act",
            "meta test\nverb calls_private() -> Int { return private_value() - 42; }\n",
        )],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("math::calls_private ... ok"), "stdout: {stdout}");
}

#[test]
fn a_module_test_sees_the_open_verbs_of_a_submodule() {
    let root = project(
        "module-submodule-open",
        &[(
            "src/math/tests/inner.act",
            "meta test\nverb calls_inner_open() -> Int { return inner_open() - 5; }\n",
        )],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("math::calls_inner_open ... ok"), "stdout: {stdout}");
}

#[test]
fn a_module_test_cannot_call_a_private_verb_of_a_submodule() {
    let root = project(
        "module-submodule-private",
        &[(
            "src/math/tests/inner.act",
            "meta test\nverb calls_inner_private() -> Int { return inner_private(); }\n",
        )],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1119"), "stderr: {stderr}");
    assert!(stderr.contains("inner_private"), "stderr: {stderr}");
    assert!(stderr.contains("src/math/tests/inner.act"), "stderr: {stderr}");
}

#[test]
fn a_project_test_cannot_call_a_private_verb() {
    let root = project(
        "project-private",
        &[(
            "tests/private.act",
            "import { open math };\nmeta test\nverb calls_private() -> Int { return private_value(); }\n",
        )],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1109"), "stderr: {stderr}");
    assert!(stderr.contains("private_value"), "stderr: {stderr}");
}

#[test]
fn a_project_test_can_call_an_open_verb() {
    let root = project(
        "project-open",
        &[(
            "tests/open.act",
            "import { open math };\nmeta test\nverb calls_open() -> Int { return public_value() - 1; }\n",
        )],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("tests::calls_open ... ok"), "stdout: {stdout}");
}

#[test]
fn files_of_a_module_tests_directory_share_helpers() {
    let root = project(
        "module-helpers",
        &[
            ("src/math/tests/helpers.act", "verb expected() -> Int { return 42; }\n"),
            (
                "src/math/tests/uses.act",
                "meta test\nverb uses_helper() -> Int { return private_value() - expected(); }\n",
            ),
        ],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("math::uses_helper ... ok"), "stdout: {stdout}");
}

#[test]
fn files_of_the_project_tests_directory_share_helpers() {
    let root = project(
        "project-helpers",
        &[
            ("tests/helpers.act", "import { open math };\nverb expected() -> Int { return 1; }\n"),
            (
                "tests/uses.act",
                "import { open math };\nmeta test\nverb uses_helper() -> Int { return public_value() - expected(); }\n",
            ),
        ],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("tests::uses_helper ... ok"), "stdout: {stdout}");
}

#[test]
fn project_test_files_importing_one_module_share_one_import() {
    // The first file imports `math` qualified, the second opens it; the shared
    // namespace holds one import, open because one of the files opens it.
    let root = project(
        "project-imports",
        &[
            (
                "tests/a_qualified.act",
                "import { math };\nmeta test\nverb qualified() -> Int { return math::public_value() - 1; }\n",
            ),
            (
                "tests/b_open.act",
                "import { open math };\nmeta test\nverb unqualified() -> Int { return public_value() - 1; }\n",
            ),
        ],
    );
    let stdout = passing(&root, &["test"]);
    assert!(stdout.contains("2 passed; 0 failed"), "stdout: {stdout}");
}

#[test]
fn a_name_declared_in_two_files_of_a_module_tests_directory_is_rejected() {
    let root = project(
        "module-duplicate",
        &[
            (
                "src/math/tests/one.act",
                "verb helper() -> Int { return 1; }\nmeta test\nverb first() -> Int { return 0; }\n",
            ),
            (
                "src/math/tests/two.act",
                "verb helper() -> Int { return 2; }\nmeta test\nverb second() -> Int { return 0; }\n",
            ),
        ],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1106"), "stderr: {stderr}");
    assert!(stderr.contains("`helper`"), "stderr: {stderr}");
    assert!(stderr.contains("src/math/tests/one.act"), "stderr: {stderr}");
    assert!(stderr.contains("src/math/tests/two.act"), "stderr: {stderr}");
}

#[test]
fn a_test_file_cannot_redeclare_a_name_of_its_module() {
    let root = project(
        "module-redeclare",
        &[(
            "src/math/tests/one.act",
            "verb private_value() -> Int { return 1; }\nmeta test\nverb first() -> Int { return 0; }\n",
        )],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1106"), "stderr: {stderr}");
    assert!(stderr.contains("src/math/api.act"), "stderr: {stderr}");
    assert!(stderr.contains("src/math/tests/one.act"), "stderr: {stderr}");
}

#[test]
fn a_name_declared_in_two_project_test_files_is_rejected() {
    let root = project(
        "project-duplicate",
        &[
            (
                "tests/one.act",
                "verb helper() -> Int { return 1; }\nmeta test\nverb first() -> Int { return 0; }\n",
            ),
            (
                "tests/two.act",
                "verb helper() -> Int { return 2; }\nmeta test\nverb second() -> Int { return 0; }\n",
            ),
        ],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1106"), "stderr: {stderr}");
    assert!(stderr.contains("`helper`"), "stderr: {stderr}");
    assert!(stderr.contains("tests/one.act") && stderr.contains("tests/two.act"), "{stderr}");
}

#[test]
fn a_meta_test_verb_in_a_production_file_names_the_module_tests_directory() {
    let root = project(
        "inline-module",
        &[("src/math/inline.act", "meta test\nverb inline() -> Int { return 0; }\n")],
    );
    for command in ["test", "check", "build"] {
        let stderr = failing(&root, &[command]);
        assert!(stderr.contains("E1144"), "{command}: {stderr}");
        assert!(stderr.contains("src/math/inline.act"), "{command}: {stderr}");
        assert!(stderr.contains("src/math/tests"), "{command}: {stderr}");
    }
}

#[test]
fn a_meta_test_verb_in_the_root_unit_is_rejected() {
    let root = project(
        "inline-root",
        &[(
            "src/main.act",
            "import { open math };\nverb main() -> Int { return 0; }\nmeta test\nverb inline() -> Int { return 0; }\n",
        )],
    );
    for command in ["test", "check", "build"] {
        let stderr = failing(&root, &[command]);
        assert!(stderr.contains("E1144"), "{command}: {stderr}");
        assert!(stderr.contains("root unit has no tests directory"), "{command}: {stderr}");
    }
}

#[test]
fn a_mirrored_project_test_names_the_module_tests_directory_it_belongs_in() {
    let root = project(
        "mirror-layout",
        &[("tests/math/api.act", "meta test\nverb old_mirror() -> Int { return 0; }\n")],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1143"), "stderr: {stderr}");
    assert!(stderr.contains("tests/math/api.act"), "stderr: {stderr}");
    assert!(stderr.contains("src/math/tests/api.act"), "stderr: {stderr}");
}

#[test]
fn a_subdirectory_of_a_module_tests_directory_is_rejected() {
    let root = project(
        "nested-tests",
        &[("src/math/tests/deep/case.act", "meta test\nverb deep() -> Int { return 0; }\n")],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1143"), "stderr: {stderr}");
    assert!(stderr.contains("src/math/tests/deep/case.act"), "stderr: {stderr}");
    assert!(stderr.contains("no subdirectories"), "stderr: {stderr}");
}

#[test]
fn a_root_unit_tests_directory_is_rejected() {
    let root = project(
        "root-tests",
        &[("src/tests/case.act", "meta test\nverb rooted() -> Int { return 0; }\n")],
    );
    let stderr = failing(&root, &["test"]);
    assert!(stderr.contains("E1143"), "stderr: {stderr}");
    assert!(stderr.contains("root unit has no tests directory"), "stderr: {stderr}");
}

#[test]
fn a_tests_directory_with_a_facade_is_never_a_module() {
    let root = project(
        "tests-facade",
        &[("src/math/tests/tests.act", "meta test\nverb faced() -> Int { return 0; }\n")],
    );
    for command in ["test", "check", "build"] {
        let stderr = failing(&root, &[command]);
        assert!(stderr.contains("E1143"), "{command}: {stderr}");
        assert!(stderr.contains("never a module"), "{command}: {stderr}");
    }
}

#[test]
fn build_check_and_run_ignore_tests_directories() {
    let root = project(
        "ignored",
        &[
            // Neither file compiles as production source.
            (
                "src/math/tests/broken.act",
                "meta test\nverb broken() -> Int { return missing(); }\n",
            ),
            ("src/math/tests/garbage.act", "this is not actus {{{\n"),
        ],
    );
    for command in ["check", "build"] {
        let output = actus(&root, &[command]);
        let (stdout, stderr) = text(&output);
        assert!(output.status.success(), "{command} stdout: {stdout}\nstderr: {stderr}");
    }
    let run = actus(&root, &["run"]);
    assert_eq!(run.status.code(), Some(0), "stderr: {}", text(&run).1);
}

#[test]
fn fmt_formats_module_test_files() {
    let root = project(
        "fmt",
        &[("src/math/tests/messy.act", "meta test\nverb messy()->Int{return 0;}\n")],
    );
    let drift = failing(&root, &["fmt", "--check"]);
    assert!(drift.contains("messy.act"), "stderr: {drift}");
    assert!(actus(&root, &["fmt"]).status.success());
    assert!(actus(&root, &["fmt", "--check"]).status.success());
    let formatted = fs::read_to_string(root.join("src/math/tests/messy.act")).expect("read");
    assert!(formatted.contains("verb messy() -> Int {"), "formatted: {formatted}");
}

#[test]
fn filters_select_by_module_file_name_and_exactness() {
    let root = project(
        "filters",
        &[
            ("src/math/tests/a.act", "meta test\nverb alpha() -> Int { return 0; }\n"),
            ("src/math/tests/b.act", "meta test\nverb beta() -> Int { return 0; }\n"),
            ("tests/c.act", "meta test\nverb gamma() -> Int { return 0; }\n"),
        ],
    );
    let list = |arguments: &[&str]| {
        let mut command = vec!["test", "--list"];
        command.extend_from_slice(arguments);
        passing(&root, &command)
    };
    let by_module = list(&["--module", "math"]);
    assert!(by_module.contains("math::alpha") && by_module.contains("math::beta"), "{by_module}");
    assert!(!by_module.contains("tests::gamma"), "{by_module}");
    let by_project_module = list(&["--module", "tests"]);
    assert!(by_project_module.contains("tests::gamma"), "{by_project_module}");
    assert!(!by_project_module.contains("math::alpha"), "{by_project_module}");
    let by_file = list(&["--file", "math/tests/b.act"]);
    assert!(by_file.contains("math::beta") && !by_file.contains("math::alpha"), "{by_file}");
    let by_name = list(&["alpha"]);
    assert!(by_name.contains("math::alpha") && !by_name.contains("math::beta"), "{by_name}");
    let exact = list(&["math::alpha", "--exact"]);
    assert!(exact.contains("math::alpha") && !exact.contains("math::beta"), "{exact}");
    let none = list(&["alph", "--exact"]);
    assert!(!none.contains("math::alpha"), "{none}");
}

#[test]
fn the_mirror_option_is_removed() {
    let root = project("mirror-option", &[]);
    let output = actus(&root, &["test", "--mirror"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output).1.contains("unknown option `--mirror`"), "{}", text(&output).1);
}

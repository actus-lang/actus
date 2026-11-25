//! E1109 help names the actual reason a declaration is not exported and the
//! concrete edit: mark it `open`, or change the facade that hides it
//! (ADR-0111 `open unit`, a missing `open`, or a nested facade chain).
//! `check` and `build` agree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MAIN: &str = "import m;\nverb main() -> Int {\n    return m::f();\n}\n";
const OPEN_F: &str = "open verb f() -> Int { return 1; }\n";

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-facade-help-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"facade-help\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write lockfile");
    fs::write(root.join("src/main.act"), MAIN).expect("write main");
    for (path, source) in files {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
        fs::write(path, source).expect("write source");
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

/// `check` and `build` must both fail with E1109 and end with `help`.
fn assert_help(name: &str, files: &[(&str, &str)], help: &str) {
    let root = project(name, files);
    for command in [
        vec!["check", "--strict", "src/main.act"],
        vec!["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"],
    ] {
        let output = actus(&root, &command);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(!output.status.success(), "{name}: {} accepted: {stderr}", command[0]);
        assert!(stderr.contains("error[E1109] at 3:12:"), "{name}: {stderr}");
        assert!(
            stderr.contains(
                "private declaration `f` from module `m` is not exported by facade `src/m/m.act`"
            ),
            "{name}: {stderr}"
        );
        assert!(stderr.trim_end().ends_with(&format!("help: {help}")), "{name}: {stderr}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn declaration_not_open_asks_to_mark_it_open() {
    assert_help(
        "not-open",
        &[("m/m.act", "open api;\n"), ("m/api.act", "verb f() -> Int { return 1; }\n")],
        "mark `f` as `open` in its module to export it",
    );
}

#[test]
fn open_declaration_in_unit_only_file_points_at_the_facade() {
    assert_help(
        "unit-only",
        &[("m/m.act", "open unit api;\n"), ("m/api.act", OPEN_F)],
        "`f` is already `open`, but the facade exports `api` only to its unit; to export it, change `open unit api;` to `open api;` in `src/m/m.act`",
    );
}

#[test]
fn open_declaration_in_unlisted_file_asks_to_open_the_file() {
    assert_help(
        "unlisted",
        &[
            ("m/m.act", "open other;\n"),
            ("m/api.act", OPEN_F),
            ("m/other.act", "open verb g() -> Int { return 1; }\n"),
        ],
        "`f` is already `open`, but the facade does not export `api`; to export it, add `open api;` to `src/m/m.act`",
    );
}

#[test]
fn nested_facade_that_limits_its_file_names_the_nested_facade() {
    assert_help(
        "nested-unit-only",
        &[("m/m.act", "open n;\n"), ("m/n/n.act", "open unit api;\n"), ("m/n/api.act", OPEN_F)],
        "`f` is already `open`, but the facade exports `api` only to its unit; to export it, change `open unit api;` to `open api;` in `src/m/n/n.act`",
    );
}

#[test]
fn nested_facade_that_omits_its_file_names_the_nested_facade() {
    assert_help(
        "nested-unlisted",
        &[("m/m.act", "open n;\n"), ("m/n/n.act", "\n"), ("m/n/api.act", OPEN_F)],
        "`f` is already `open`, but the facade does not export `api`; to export it, add `open api;` to `src/m/n/n.act`",
    );
}

#[test]
fn parent_facade_that_opens_its_child_for_the_unit_only_names_the_parent() {
    assert_help(
        "parent-unit-only",
        &[("m/m.act", "open unit n;\n"), ("m/n/n.act", "open api;\n"), ("m/n/api.act", OPEN_F)],
        "`f` is already `open`, but the facade exports `n` only to its unit; to export it, change `open unit n;` to `open n;` in `src/m/m.act`",
    );
}

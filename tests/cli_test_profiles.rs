//! ADR-0122: `actus test` accepts the profile options of `actus build`. The
//! profile changes how the test executables are optimized, never what a test
//! means.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MANIFEST: &str = "[package]\nname = \"profiles\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n";
const PASSES: &str = "meta test\nverb sums() -> Int {\n    erg total: Int = 0;\n    for erg step in 0..1000 { total = total + step; }\n    if total == 499500 { return 0; }\n    return 1;\n}\n";
const OVERFLOW: &str = "meta test\nverb before() -> Int { return 0; }\nmeta test\nverb overflows() -> Int {\n    erg x: u8 = 250u8;\n    erg y: u8 = x + 10u8;\n    return y as Int;\n}\nmeta test\nverb after() -> Int { return 0; }\n";

/// A project whose manifest is `MANIFEST` followed by `extra`, with one test
/// file `tests/a.act`.
fn project(name: &str, extra: &str, source: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-profiles-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("tests")).expect("create tests directory");
    fs::write(root.join("Actus.toml"), format!("{MANIFEST}{extra}")).expect("write manifest");
    fs::write(root.join("tests/a.act"), source).expect("write test file");
    root
}

fn actus(root: &Path, arguments: &[&str], environment: &[(&str, &str)]) -> (Output, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .envs(environment.iter().copied())
        .current_dir(root)
        .output()
        .expect("run actus");
    let stdout = String::from_utf8_lossy(&output.stdout).replace('\\', "/");
    (output, stdout)
}

/// Runs `actus test` keeping its artifacts under `<root>/tmp`; returns the
/// stdout and the bytes of the one executable the run built.
fn keeping(root: &Path, arguments: &[&str]) -> (String, Vec<u8>) {
    let tmp = root.join("tmp");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).expect("create TMPDIR");
    let tmp_text = tmp.to_string_lossy().into_owned();
    let mut command = vec!["test"];
    command.extend_from_slice(arguments);
    let (output, stdout) =
        actus(root, &command, &[("TMPDIR", &tmp_text), ("ACTUS_TEST_KEEP_ARTIFACTS", "1")]);
    assert!(output.status.success(), "stdout: {stdout}");
    let executables: Vec<PathBuf> = fs::read_dir(&tmp)
        .expect("read TMPDIR")
        .flatten()
        .map(|entry| entry.path().join("test.bin"))
        .filter(|path| path.is_file())
        .collect();
    assert_eq!(executables.len(), 1, "one executable per run; stdout: {stdout}");
    (stdout, fs::read(&executables[0]).expect("read test executable"))
}

fn backends() -> Vec<&'static str> {
    let mut backends = Vec::new();
    if cfg!(feature = "llvm") {
        backends.push("llvm");
    }
    if cfg!(feature = "cranelift") {
        backends.push("cranelift");
    }
    backends
}

fn first_line(stdout: &str) -> &str {
    stdout.lines().next().expect("a header line")
}

#[test]
fn release_run_names_the_release_profile_in_the_header() {
    let root = project("header-release", "", PASSES);
    let (output, stdout) = actus(&root, &["test", "--release"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert_eq!(first_line(&stdout), "running 1 test (release) (0 filtered)");
    assert!(stdout.contains("1 passed; 0 failed"), "stdout: {stdout}");
}

#[test]
fn a_named_profile_is_named_in_the_header() {
    let root = project("header-size", "", PASSES);
    let (output, stdout) = actus(&root, &["test", "--profile", "size"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert_eq!(first_line(&stdout), "running 1 test (size) (0 filtered)");
}

#[test]
fn the_debug_header_does_not_name_a_profile() {
    let root = project("header-debug", "", PASSES);
    for arguments in [&["test"][..], &["test", "--profile", "debug"]] {
        let (output, stdout) = actus(&root, arguments, &[]);
        assert!(output.status.success(), "stdout: {stdout}");
        assert_eq!(first_line(&stdout), "running 1 test (0 filtered)", "{arguments:?}");
    }
}

#[test]
fn a_strict_release_run_names_both_in_the_header() {
    let root = project("header-strict", "", PASSES);
    let (output, stdout) = actus(&root, &["test", "--strict", "--release"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert_eq!(first_line(&stdout), "running 1 test (strict) (release) (0 filtered)");
}

/// `actus test` and `actus build` fail with the same status and message.
fn assert_same_error_as_build(name: &str, options: &[&str], message: &str) {
    let root = project(name, "", PASSES);
    let build_arguments: Vec<&str> = ["build"].iter().chain(options).copied().collect();
    let test_arguments: Vec<&str> = ["test"].iter().chain(options).copied().collect();
    let build = actus(&root, &build_arguments, &[]).0;
    let test = actus(&root, &test_arguments, &[]).0;
    let build_error = String::from_utf8_lossy(&build.stderr).into_owned();
    let test_error = String::from_utf8_lossy(&test.stderr).into_owned();
    assert!(test_error.contains(message), "stderr: {test_error}");
    assert_eq!(test_error, build_error);
    assert_eq!(test.status.code(), Some(2));
    assert_eq!(test.status.code(), build.status.code());
    assert!(test.stdout.is_empty(), "no header before the options are accepted");
}

#[test]
fn release_and_profile_together_are_the_error_of_build() {
    assert_same_error_as_build(
        "conflict",
        &["--release", "--profile", "size"],
        "duplicate or conflicting profile option",
    );
}

#[test]
fn an_unknown_profile_is_the_error_of_build() {
    assert_same_error_as_build(
        "unknown",
        &["--profile", "fast"],
        "unsupported build profile `fast`",
    );
}

#[test]
fn a_profile_option_without_a_name_is_the_error_of_build() {
    assert_same_error_as_build("missing", &["--profile"], "missing value after `--profile`");
}

#[test]
fn run_and_watch_reject_profile_options_like_build() {
    for command in ["run", "watch"] {
        for (options, message) in [
            (&["--release", "--release"][..], "duplicate or conflicting profile option"),
            (&["--release", "--profile", "size"], "duplicate or conflicting profile option"),
            (&["--profile", "fast"], "unsupported build profile `fast`"),
            (&["--profile"], "missing value after `--profile`"),
        ] {
            let root = project(&format!("{command}-options"), "", PASSES);
            let mut build = vec!["build"];
            build.extend_from_slice(options);
            let mut other = vec![command];
            other.extend_from_slice(options);
            let build = actus(&root, &build, &[]).0;
            let other = actus(&root, &other, &[]).0;
            let context = format!("{command} {options:?}");
            let error = String::from_utf8_lossy(&other.stderr).into_owned();
            assert!(error.contains(message), "{context}; stderr: {error}");
            assert_eq!(error, String::from_utf8_lossy(&build.stderr), "{context}");
            assert_eq!(other.status.code(), build.status.code(), "{context}");
            assert_eq!(other.status.code(), Some(2), "{context}");
        }
    }
}

#[test]
fn the_manifest_build_profile_is_the_default_profile() {
    let root = project("manifest-default", "\n[build]\nprofile = \"release\"\n", PASSES);
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert_eq!(first_line(&stdout), "running 1 test (release) (0 filtered)");
}

#[test]
fn an_explicit_debug_profile_overrides_the_manifest_build_profile() {
    let root = project("manifest-override", "\n[build]\nprofile = \"release\"\n", PASSES);
    let (output, stdout) = actus(&root, &["test", "--profile", "debug"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert_eq!(first_line(&stdout), "running 1 test (0 filtered)");
}

#[test]
fn a_profile_name_is_not_taken_for_a_filter() {
    let root = project("filter", "", PASSES);
    let (output, stdout) = actus(&root, &["test", "--profile", "release", "sums"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert_eq!(first_line(&stdout), "running 1 test (release) (0 filtered)");
}

#[test]
fn a_runtime_check_traps_in_release_as_in_debug_on_every_backend() {
    for backend in backends() {
        let root = project(
            &format!("trap-{backend}"),
            &format!("\n[build]\nbackend = \"{backend}\"\n"),
            OVERFLOW,
        );
        for arguments in [&["test"][..], &["test", "--release"], &["test", "--profile", "size"]] {
            let (output, stdout) = actus(&root, arguments, &[]);
            let context = format!("{backend} {arguments:?}; stdout: {stdout}");
            assert_eq!(output.status.code(), Some(1), "{context}");
            let trapped = stdout
                .lines()
                .find(|line| line.starts_with("test tests::overflows "))
                .unwrap_or_else(|| panic!("no line for the trapping test; {context}"));
            assert!(trapped.contains("FAILED") && trapped.contains("exit code 101"), "{context}");
            assert!(stdout.contains("integer overflow at tests/a.act:"), "{context}");
            assert!(stdout.contains("2 passed; 1 failed"), "{context}");
        }
    }
}

#[test]
fn the_profile_optimizes_the_test_executable_on_every_backend() {
    for backend in backends() {
        let root = project(
            &format!("levels-{backend}"),
            &format!("\n[build]\nbackend = \"{backend}\"\n"),
            PASSES,
        );
        let debug = keeping(&root, &[]).1;
        let explicit = keeping(&root, &["--profile", "debug"]).1;
        let release = keeping(&root, &["--release"]).1;
        let size = keeping(&root, &["--profile", "size"]).1;
        assert!(debug == explicit, "{backend}: `--profile debug` is the default build");
        assert!(debug != release, "{backend}: release builds a different executable");
        assert!(debug != size, "{backend}: size builds a different executable");
    }
}

#[test]
fn a_manifest_profile_table_sets_the_level_of_the_selected_profile() {
    let root = project("manifest", "\n[profile.release]\nopt_level = \"none\"\n", PASSES);
    let debug = keeping(&root, &[]).1;
    let (stdout, release) = keeping(&root, &["--release"]);
    assert_eq!(first_line(&stdout), "running 1 test (release) (0 filtered)");
    assert!(debug == release, "`opt_level = \"none\"` makes the release profile build at O0");
}

#[test]
fn a_manifest_profile_table_does_not_leak_into_other_profiles() {
    let root = project("manifest-other", "\n[profile.size]\nopt_level = \"none\"\n", PASSES);
    let debug = keeping(&root, &[]).1;
    let release = keeping(&root, &["--release"]).1;
    let size = keeping(&root, &["--profile", "size"]).1;
    assert!(debug == size, "size at `none` is the debug build");
    assert!(debug != release, "release keeps its own level");
}

#[test]
fn the_timeout_applies_in_release() {
    let root = project(
        "timeout",
        "",
        "meta test\nverb slow() -> Int { loop { } }\nmeta test\nverb quick() -> Int { return 0; }\n",
    );
    let (output, stdout) =
        actus(&root, &["test", "--release"], &[("ACTUS_TEST_TIMEOUT_SECONDS", "2")]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    let slow = stdout.lines().find(|line| line.starts_with("test tests::slow ")).expect("slow");
    assert!(slow.contains("FAILED"), "stdout: {stdout}");
    let quick = stdout.lines().find(|line| line.starts_with("test tests::quick ")).expect("quick");
    assert!(quick.contains("... ok"), "stdout: {stdout}");
}

#[test]
fn a_non_zero_status_fails_the_test_in_release() {
    let root = project("status", "", "meta test\nverb bad() -> Int { return 7; }\n");
    let (output, stdout) = actus(&root, &["test", "--release"], &[]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(stdout.contains("exit code 7"), "stdout: {stdout}");
}

/// Copies the standard library into the project so tests can import `fs` and
/// `path`.
fn copy_standard_library(root: &Path) {
    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("create directory");
        for entry in fs::read_dir(from).expect("read directory").flatten() {
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).expect("copy file");
            }
        }
    }
    let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std");
    let manifest = fs::read_to_string(library.join("Actus.toml")).expect("std manifest");
    fs::write(root.join("Actus.toml"), manifest.replace("name = \"std\"", "name = \"profiles\""))
        .expect("write manifest");
    let _ = fs::remove_dir_all(root.join("src"));
    copy_tree(&library.join("src"), &root.join("src"));
}

/// Tests that require `f` not to exist in their working directory and then
/// create it, optionally under the injected `write_failure` fault.
fn marker_tests(fault: bool) -> String {
    let attribute = if fault { "meta fault(\"write_failure\")\n" } else { "" };
    let expected = if fault { 3 } else { 0 };
    let mut source = String::from(
        "import {\n    fs,\n    path,\n};\n\nmeta effect(\"filesystem\", \"blocking\")\nverb marker_status() -> Int {\n    erg raw: Buffer = Buffer[0];\n    append(raw, 102u8);\n    append(raw, 0u8);\n    erg made = path::path_from_ascii(storage: dat raw);\n    return case dat made {\n        Result.Err(_) => 1,\n        Result.Ok(target) => {\n            erg before = fs::read_to_bytes(path: abs target);\n            erg existed: Bool = false;\n            case dat before {\n                Result.Ok(bytes) => {\n                    existed = true;\n                    drop(bytes);\n                },\n                Result.Err(_) => {},\n            };\n            if existed {\n                drop(target);\n                return 2;\n            }\n            erg payload: Buffer = Buffer[0];\n            append(payload, 120u8);\n            erg written = fs::write_file(path: abs target, contents: abs payload);\n            drop(target);\n            drop(payload);\n            return case dat written {\n                Result.Ok(_) => 0,\n                Result.Err(_) => 3,\n            };\n        },\n    };\n}\n\n",
    );
    for name in ["first", "second"] {
        source.push_str(&format!(
            "meta test\n{attribute}meta effect(\"filesystem\", \"blocking\")\nverb {name}_writes() -> Int {{\n    erg status = marker_status();\n    if status == {expected} {{ return 0; }}\n    return 9;\n}}\n\n"
        ));
    }
    source
}

#[test]
fn each_test_starts_in_an_empty_working_directory_in_release() {
    let root = project("cwd", "", "");
    copy_standard_library(&root);
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("tests/io.act"), marker_tests(false)).expect("write test file");
    let (output, stdout) = actus(&root, &["test", "--release", "--test-threads", "1"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("2 passed; 0 failed"), "stdout: {stdout}");
}

#[cfg(feature = "test-fault-injection")]
#[test]
fn an_injected_fault_behaves_the_same_in_release() {
    let root = project("fault", "", "");
    copy_standard_library(&root);
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("tests/io.act"), marker_tests(true)).expect("write test file");
    for arguments in [&["test"][..], &["test", "--release"]] {
        let (output, stdout) = actus(&root, arguments, &[]);
        assert!(output.status.success(), "{arguments:?}; stdout: {stdout}");
        assert!(stdout.contains("2 passed; 0 failed"), "{arguments:?}; stdout: {stdout}");
    }
}

#[test]
fn the_help_text_lists_the_profile_options() {
    let (output, stdout) = actus(Path::new("."), &["test", "--help"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("[--release|--profile <name>]"), "stdout: {stdout}");
}

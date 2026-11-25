//! ADR-0121: `actus test` builds one executable per test directory and starts
//! it once per test, each start in its own process.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const OVERFLOW: &str = "meta test\nverb overflows() -> Int {\n    erg x: u8 = 250u8;\n    erg y: u8 = x + 10u8;\n    return y as Int;\n}\n";

/// A project with the modules `math` and `shapes`; `files` are written
/// relative to the project root.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-groups-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let base = [
        ("Actus.toml", "[package]\nname = \"groups\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n"),
        ("src/main.act", "import { open math, open shapes };\nverb main() -> Int { return 0; }\n"),
        ("src/math/math.act", "open api;\n"),
        ("src/math/api.act", "open verb math_value() -> Int { return 3; }\n"),
        ("src/shapes/shapes.act", "open api;\n"),
        ("src/shapes/api.act", "open verb shapes_value() -> Int { return 4; }\n"),
    ];
    for (path, source) in base.iter().chain(files) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
        fs::write(path, source).expect("write fixture file");
    }
    root
}

fn actus(root: &Path, arguments: &[&str], environment: &[(&str, &str)]) -> (Output, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .envs(environment.iter().copied())
        .current_dir(root)
        .output()
        .expect("run actus");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

/// Runs `actus test` with `TMPDIR` set to `<root>/tmp` and the artifacts kept,
/// so the executables of the run can be inspected afterwards.
fn keeping(root: &Path, arguments: &[&str]) -> (Output, String) {
    let tmp = root.join("tmp");
    fs::create_dir_all(&tmp).expect("create TMPDIR");
    let tmp = tmp.to_string_lossy().into_owned();
    actus(root, arguments, &[("TMPDIR", &tmp), ("ACTUS_TEST_KEEP_ARTIFACTS", "1")])
}

/// The directories of a kept run that hold a shared executable, and the
/// working directories of the tests that ran it.
fn kept_artifacts(root: &Path) -> (Vec<PathBuf>, usize) {
    let mut executables = Vec::new();
    let mut working_directories = 0;
    for entry in fs::read_dir(root.join("tmp")).expect("read TMPDIR").flatten() {
        let path = entry.path();
        if !entry.file_name().to_string_lossy().starts_with("actus-test-") {
            continue;
        }
        if path.join("test.bin").is_file() {
            executables.push(path.clone());
        }
        working_directories += fs::read_dir(&path)
            .expect("read test directory")
            .flatten()
            .filter(|child| child.file_name().to_string_lossy().starts_with("run-"))
            .count();
    }
    (executables, working_directories)
}

fn test_line(stdout: &str, name: &str) -> String {
    stdout
        .lines()
        .find(|line| line.starts_with(&format!("test {name} ")))
        .unwrap_or_else(|| panic!("no line for {name}; stdout: {stdout}"))
        .to_owned()
}

#[test]
fn each_test_directory_is_built_into_one_executable() {
    let root = project(
        "one-executable",
        &[
            (
                "src/math/tests/first.act",
                "meta test\nverb one() -> Int { return math_value() - 3; }\n",
            ),
            (
                "src/math/tests/second.act",
                "meta test\nverb two() -> Int { return 0; }\nmeta test\nverb three() -> Int { return 0; }\n",
            ),
            (
                "src/shapes/tests/all.act",
                "meta test\nverb four() -> Int { return shapes_value() - 4; }\nmeta test\nverb five() -> Int { return 0; }\n",
            ),
            (
                "tests/api.act",
                "meta test\nverb six() -> Int { return 0; }\nmeta test\nverb seven() -> Int { return 0; }\n",
            ),
        ],
    );
    let (output, stdout) = keeping(&root, &["test"]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("7 passed; 0 failed"), "stdout: {stdout}");
    let (executables, working_directories) = kept_artifacts(&root);
    assert_eq!(executables.len(), 3, "one executable per test directory: {executables:?}");
    assert_eq!(working_directories, 7, "one working directory per test");
}

#[test]
fn a_trivial_test_executable_does_not_link_the_actus_runtime() {
    // Selecting a test uses the C library only; with the runtime linked the
    // executable is several megabytes instead of some kilobytes.
    let root = project("small", &[("tests/a.act", "meta test\nverb one() -> Int { return 0; }\n")]);
    let (output, stdout) = keeping(&root, &["test"]);
    assert!(output.status.success(), "stdout: {stdout}");
    let (executables, _) = kept_artifacts(&root);
    let size = fs::metadata(executables[0].join("test.bin")).expect("executable").len();
    assert!(size < 200_000, "test executable is {size} bytes");
}

#[test]
fn a_test_working_directory_is_empty_when_the_test_starts() {
    // The tests write nothing, so whatever a kept working directory holds was
    // put there by the runner.
    let root = project(
        "empty-directory",
        &[(
            "tests/a.act",
            "meta test\nverb one() -> Int { return 0; }\nmeta test\nverb two() -> Int { return 0; }\n",
        )],
    );
    let (output, stdout) = keeping(&root, &["test"]);
    assert!(output.status.success(), "stdout: {stdout}");
    let mut checked = 0;
    for entry in fs::read_dir(root.join("tmp")).expect("read TMPDIR").flatten() {
        for child in fs::read_dir(entry.path()).expect("read test directory").flatten() {
            if child.file_name().to_string_lossy().starts_with("run-") {
                let contents = fs::read_dir(child.path()).expect("read working directory").count();
                assert_eq!(contents, 0, "{} is not empty", child.path().display());
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 2);
}

#[test]
fn a_trapping_test_fails_alone_and_its_siblings_pass() {
    let root = project(
        "trap",
        &[(
            "tests/a.act",
            &format!(
                "meta test\nverb before() -> Int {{ return 0; }}\n{OVERFLOW}meta test\nverb after() -> Int {{ return 0; }}\n"
            ),
        )],
    );
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::before").contains("... ok"), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::after").contains("... ok"), "stdout: {stdout}");
    let trapped = test_line(&stdout, "tests::overflows");
    assert!(trapped.contains("FAILED") && trapped.contains("exit code 101"), "stdout: {stdout}");
    assert!(stdout.contains("integer overflow at tests/a.act:"), "stdout: {stdout}");
    assert!(stdout.contains("2 passed; 1 failed"), "stdout: {stdout}");
}

#[test]
fn a_test_returning_a_nonzero_status_fails_alone() {
    let root = project(
        "nonzero",
        &[(
            "tests/a.act",
            "meta test\nverb good() -> Int { return 0; }\nmeta test\nverb bad() -> Int { return 7; }\nmeta test\nverb fine() -> Int { return 0; }\n",
        )],
    );
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::good").contains("... ok"), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::fine").contains("... ok"), "stdout: {stdout}");
    assert!(
        test_line(&stdout, "tests::bad").contains("FAILED")
            && test_line(&stdout, "tests::bad").contains("exit code 7"),
        "stdout: {stdout}"
    );
}

#[test]
fn a_timeout_in_one_test_does_not_affect_its_siblings() {
    let root = project(
        "timeout",
        &[(
            "tests/a.act",
            "meta test\nverb early() -> Int { return 0; }\nmeta test\nverb hangs() -> Int { loop { } }\nmeta test\nverb late() -> Int { return 0; }\n",
        )],
    );
    let (output, stdout) = actus(&root, &["test"], &[("ACTUS_TEST_TIMEOUT_SECONDS", "2")]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::early").contains("... ok"), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::late").contains("... ok"), "stdout: {stdout}");
    assert!(
        test_line(&stdout, "tests::hangs").contains("timed out after 2 seconds"),
        "stdout: {stdout}"
    );
}

#[test]
fn a_filter_runs_only_the_selected_test_of_a_group() {
    let root = project(
        "filter",
        &[(
            "tests/a.act",
            "meta test\nverb wanted() -> Int { return 0; }\nmeta test\nverb unwanted() -> Int { return 7; }\n",
        )],
    );
    let (output, stdout) = keeping(&root, &["test", "wanted", "--exact"]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("running 1 test (1 filtered)"), "stdout: {stdout}");
    assert!(stdout.contains("tests::wanted ... ok"), "stdout: {stdout}");
    assert!(!stdout.contains("unwanted"), "stdout: {stdout}");
    assert_eq!(kept_artifacts(&root).1, 1, "only the selected test ran");
}

#[test]
fn a_group_that_fails_to_build_fails_its_tests_and_other_groups_still_run() {
    let root = project(
        "broken-group",
        &[
            (
                "src/math/tests/a.act",
                "verb main() -> Int { return 0; }\nmeta test\nverb first() -> Int { return 0; }\nmeta test\nverb second() -> Int { return 0; }\n",
            ),
            ("src/shapes/tests/a.act", "meta test\nverb healthy() -> Int { return 0; }\n"),
        ],
    );
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(test_line(&stdout, "math::first").contains("FAILED"), "stdout: {stdout}");
    assert!(test_line(&stdout, "math::second").contains("FAILED"), "stdout: {stdout}");
    assert!(test_line(&stdout, "shapes::healthy").contains("... ok"), "stdout: {stdout}");
    assert_eq!(
        stdout.matches("already defines `main`").count(),
        1,
        "diagnostic once; stdout: {stdout}"
    );
    assert!(test_line(&stdout, "math::second").contains("failed to build"), "stdout: {stdout}");
    assert!(stdout.contains("1 passed; 2 failed"), "stdout: {stdout}");
}

#[test]
fn a_test_verb_with_parameters_fails_alone() {
    let root = project(
        "parameters",
        &[(
            "tests/a.act",
            "meta test\nverb good() -> Int { return 0; }\nmeta test\nverb takes(erg count: Int) -> Int { return count; }\n",
        )],
    );
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::good").contains("... ok"), "stdout: {stdout}");
    assert!(
        test_line(&stdout, "tests::takes").contains("must not have parameters"),
        "stdout: {stdout}"
    );
}

#[test]
fn freestanding_tests_keep_one_executable_per_test() {
    // A freestanding executable has no process environment to select a test with.
    let root = project(
        "freestanding",
        &[
            (
                "Actus.toml",
                "[package]\nname = \"groups\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n\n[build]\nentry_contract = \"freestanding\"\n",
            ),
            (
                "tests/a.act",
                "meta test\nverb one() -> Int { return 0; }\nmeta test\nverb two() -> Int { return 0; }\nmeta test\nverb three() -> Int { return 0; }\n",
            ),
        ],
    );
    let (output, stdout) = keeping(&root, &["test"]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("3 passed; 0 failed"), "stdout: {stdout}");
    assert_eq!(kept_artifacts(&root).0.len(), 3);
}

#[test]
fn results_are_reported_in_canonical_order_even_when_a_later_test_finishes_first() {
    let root = project(
        "order",
        &[
            ("src/math/tests/a.act", "meta test\nverb slow() -> Int { loop { } }\n"),
            ("src/shapes/tests/a.act", "meta test\nverb quick() -> Int { return 0; }\n"),
        ],
    );
    let (_, stdout) =
        actus(&root, &["test", "--test-threads", "2"], &[("ACTUS_TEST_TIMEOUT_SECONDS", "2")]);
    let slow = stdout.find("test math::slow").expect("slow reported");
    let quick = stdout.find("test shapes::quick").expect("quick reported");
    assert!(slow < quick, "stdout: {stdout}");
}

#[test]
fn each_test_runs_in_its_own_working_directory() {
    let root = project("cwd", &[]);
    copy_standard_library(&root);
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("tests/io.act"), WRITES_MARKER).expect("write test file");
    let (output, stdout) = actus(&root, &["test", "--test-threads", "1"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("2 passed; 0 failed"), "stdout: {stdout}");
}

#[cfg(feature = "test-fault-injection")]
#[test]
fn a_fault_is_injected_only_into_the_test_that_declares_it() {
    let root = project("fault", &[]);
    copy_standard_library(&root);
    let source = format!(
        "{WRITES_MARKER}meta test\nmeta fault(\"write_failure\")\nmeta effect(\"filesystem\", \"blocking\")\nverb faulted_write_is_rejected() -> Int {{\n    erg status = marker_status();\n    if status == 3 {{ return 0; }}\n    return 9;\n}}\nmeta test\nmeta fault(\"write_failure\")\nmeta effect(\"filesystem\", \"blocking\")\nverb second_faulted_write_is_rejected() -> Int {{\n    erg status = marker_status();\n    if status == 3 {{ return 0; }}\n    return 9;\n}}\n"
    );
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("tests/io.act"), source).expect("write test file");
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("4 passed; 0 failed"), "stdout: {stdout}");
}

#[cfg(not(feature = "test-fault-injection"))]
#[test]
fn a_fault_test_without_fault_injection_fails_alone() {
    let root = project(
        "fault-unavailable",
        &[(
            "tests/a.act",
            "meta test\nverb plain() -> Int { return 0; }\nmeta test\nmeta fault(\"write_failure\")\nverb faulted() -> Int { return 0; }\n",
        )],
    );
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(test_line(&stdout, "tests::plain").contains("... ok"), "stdout: {stdout}");
    assert!(
        test_line(&stdout, "tests::faulted").contains("--features test-fault-injection"),
        "stdout: {stdout}"
    );
}

#[test]
fn tests_with_different_contracts_share_one_executable() {
    let root = project(
        "contracts",
        &[(
            "tests/a.act",
            "meta effect(\"filesystem\", \"blocking\") verb primitive() -> Int { return 0; }\nmeta effect(\"filesystem\", \"blocking\") meta test verb with_effect() -> Int { return primitive(); }\nmeta test verb without_effect() -> Int { return 0; }\n",
        )],
    );
    let (output, stdout) = keeping(&root, &["test"]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("2 passed; 0 failed"), "stdout: {stdout}");
    assert_eq!(kept_artifacts(&root).0.len(), 1);
}

#[test]
fn listing_tests_builds_nothing() {
    let root =
        project("list", &[("tests/a.act", "meta test\nverb listed() -> Int { return 0; }\n")]);
    let (output, stdout) = keeping(&root, &["test", "--list"]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("tests::listed"), "stdout: {stdout}");
    let leftovers = fs::read_dir(root.join("tmp")).expect("read TMPDIR").count();
    assert_eq!(leftovers, 0, "--list must not build");
}

#[test]
fn artifacts_are_removed_when_the_run_is_over() {
    let root = project(
        "cleanup",
        &[(
            "tests/a.act",
            "meta test\nverb one() -> Int { return 0; }\nmeta test\nverb two() -> Int { return 0; }\n",
        )],
    );
    let tmp = root.join("tmp");
    fs::create_dir_all(&tmp).expect("create TMPDIR");
    let (output, stdout) = actus(&root, &["test"], &[("TMPDIR", &tmp.to_string_lossy())]);
    assert!(output.status.success(), "stdout: {stdout}");
    let leftovers = fs::read_dir(&tmp).expect("read TMPDIR").count();
    assert_eq!(leftovers, 0, "the executable and working directories are removed");
}

#[test]
fn a_test_that_reaches_the_entry_failure_path_is_an_internal_error_not_a_pass() {
    let root = project(
        "internal-error",
        &[(
            "tests/a.act",
            "unsafe extern \"C\" verb write(erg descriptor: Int, abs text: String, erg count: Usize) -> Int;\nunsafe extern \"C\" verb exit(erg status: Int) -> Int;\nmeta test\nverb forged() -> Int {\n    erg message: String = \"actus test entry error: forged\";\n    erg descriptor: Int = 2;\n    erg count: Usize = 30;\n    erg status: Int = 125;\n    write(descriptor: erg descriptor, text: abs message, count: erg count);\n    exit(status: erg status);\n    return status;\n}\n",
        )],
    );
    let (output, stdout) = actus(&root, &["test"], &[]);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    let line = test_line(&stdout, "tests::forged");
    assert!(line.contains("FAILED") && line.contains("internal error"), "stdout: {stdout}");
    assert!(stdout.contains("0 passed; 1 failed"), "stdout: {stdout}");
}

#[test]
fn an_executable_started_without_a_valid_test_index_exits_with_the_entry_status() {
    let root =
        project("selection", &[("tests/a.act", "meta test\nverb one() -> Int { return 0; }\n")]);
    let (output, stdout) = keeping(&root, &["test"]);
    assert!(output.status.success(), "stdout: {stdout}");
    let (executables, _) = kept_artifacts(&root);
    let executable = executables[0].join("test.bin");
    for index in [
        None,
        Some(""),
        Some("5"),
        Some("-1"),
        Some("one"),
        Some("+0"),
        Some(" 0"),
        Some("0 "),
        Some("2147483648"),
        Some("99999999999999999999"),
    ] {
        let mut command = Command::new(&executable);
        command.env_remove("ACTUS_TEST_INDEX");
        if let Some(index) = index {
            command.env("ACTUS_TEST_INDEX", index);
        }
        let output = command.output().expect("run test executable");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(125), "index {index:?}; stderr: {stderr}");
        assert!(
            stderr.starts_with("actus test entry error: "),
            "index {index:?}; stderr: {stderr}"
        );
    }
    for valid in ["0", "00", "000000000000000001"] {
        let output =
            Command::new(&executable).env("ACTUS_TEST_INDEX", valid).output().expect("run");
        let expected = if valid.ends_with('1') { 125 } else { 0 };
        assert_eq!(output.status.code(), Some(expected), "index {valid}");
    }
}

/// Two tests that each require `f` not to exist in the working directory and
/// then create it.
const WRITES_MARKER: &str = r#"import {
    fs,
    path,
};

meta effect("filesystem", "blocking")
verb marker_status() -> Int {
    erg raw: Buffer = Buffer[0];
    append(raw, 102u8);
    append(raw, 0u8);
    erg made = path::path_from_ascii(storage: dat raw);
    return case dat made {
        Result.Err(_) => 1,
        Result.Ok(target) => {
            erg before = fs::read_to_bytes(path: abs target);
            erg existed: Bool = false;
            case dat before {
                Result.Ok(bytes) => {
                    existed = true;
                    drop(bytes);
                },
                Result.Err(_) => {},
            };
            if existed {
                drop(target);
                return 2;
            }
            erg payload: Buffer = Buffer[0];
            append(payload, 120u8);
            erg written = fs::write_file(path: abs target, contents: abs payload);
            drop(target);
            drop(payload);
            return case dat written {
                Result.Ok(_) => 0,
                Result.Err(_) => 3,
            };
        },
    };
}

meta test
meta effect("filesystem", "blocking")
verb first_writes() -> Int { return marker_status(); }

meta test
meta effect("filesystem", "blocking")
verb second_writes() -> Int { return marker_status(); }

"#;

/// Makes the project a copy of the standard library, whose modules its tests
/// can import.
fn copy_standard_library(root: &Path) {
    let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std");
    let manifest = fs::read_to_string(library.join("Actus.toml")).expect("std manifest");
    fs::write(root.join("Actus.toml"), manifest.replace("name = \"std\"", "name = \"groups\""))
        .expect("write manifest");
    let _ = fs::remove_dir_all(root.join("src"));
    copy_tree(&library.join("src"), &root.join("src"));
}

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

/// Three test directories (two module directories and `tests/`) with five
/// tests; the `shapes` directory parses but cannot be compiled.
fn counted_project(name: &str) -> PathBuf {
    project(
        name,
        &[
            (
                "src/math/tests/a.act",
                "meta test\nverb add() -> Int { return 0; }\nmeta test\nverb sub() -> Int { return 0; }\n",
            ),
            (
                "src/shapes/tests/a.act",
                "meta test\nverb broken() -> Int { return missing_name(); }\n",
            ),
            (
                "tests/a.act",
                "meta test\nverb alpha() -> Int { return 0; }\nmeta test\nverb beta() -> Int { return 0; }\n",
            ),
        ],
    )
}

#[test]
fn filtered_counts_every_unselected_test_of_every_directory() {
    let root = counted_project("count-one");
    let (output, stdout) = actus(&root, &["test", "add", "--exact"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("running 1 test (4 filtered)"), "stdout: {stdout}");
    assert!(stdout.contains("math::add ... ok"), "stdout: {stdout}");
}

#[test]
fn a_filter_matching_nothing_reports_every_test_as_filtered() {
    let root = counted_project("count-none");
    let (output, stdout) = actus(&root, &["test", "no_such_test"], &[]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("running 0 tests (5 filtered)"), "stdout: {stdout}");
}

#[test]
fn an_unselected_directory_that_does_not_compile_is_not_compiled() {
    let root = counted_project("count-uncompiled");
    let (output, stdout) = keeping(&root, &["test", "alpha"]);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("running 1 test (4 filtered)"), "stdout: {stdout}");
    assert_eq!(kept_artifacts(&root).0.len(), 1, "only the selected directory was built");
    // Selecting it makes the compile error surface, so it was skipped, not ignored.
    let (output, _) = actus(&root, &["test", "broken"], &[]);
    assert!(!output.status.success());
}

#[test]
fn list_reports_the_same_counts_as_the_run_header() {
    let root = counted_project("count-list");
    let header = |arguments: &[&str]| {
        let (_, stdout) = actus(&root, arguments, &[]);
        stdout.lines().next().unwrap_or_default().to_owned()
    };
    let run = header(&["test", "beta"]);
    assert_eq!(run, "running 1 test (4 filtered)");
    assert_eq!(header(&["test", "beta", "--list"]), run);
}

#[test]
fn the_header_uses_the_singular_only_for_one_test() {
    let root = counted_project("count-plural");
    let (_, stdout) = actus(&root, &["test", "a", "--module", "tests", "--list"], &[]);
    assert!(stdout.starts_with("running 2 tests (3 filtered)"), "stdout: {stdout}");
}

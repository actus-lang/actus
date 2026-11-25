//! A reader that closes the pipe (`actus ... | head -1`) stops the command
//! quietly: no panic, no backtrace, no message, the status of a process that
//! `SIGPIPE` terminated, and no artifacts left behind.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// `128 + SIGPIPE`; the same on every platform.
const CLOSED_STREAM_STATUS: i32 = 141;

/// A private directory that is the temporary directory of the command under
/// test, so that the artifacts it leaves can be found.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("actus-closed-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tmp")).expect("create sandbox");
        Self { root }
    }

    fn temporary(&self) -> PathBuf {
        self.root.join("tmp")
    }

    fn project(&self) -> PathBuf {
        self.root.join("project")
    }

    fn leftovers(&self) -> Vec<String> {
        fs::read_dir(self.temporary())
            .expect("read sandbox temporary directory")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Runs `actus` with piped stdout. `read_lines` lines are read before the
/// reader is dropped; with 0 the pipe is closed before the command can write.
fn run_with_closed_reader(
    sandbox: &Sandbox,
    directory: &Path,
    arguments: &[&str],
    read_lines: usize,
) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(directory)
        .env("TMPDIR", sandbox.temporary())
        .env("TMP", sandbox.temporary())
        .env("TEMP", sandbox.temporary())
        .env("RUST_BACKTRACE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn actus");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut reader = BufReader::new(stdout);
    for _ in 0..read_lines {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).expect("read a line"), 0, "stdout ended early");
    }
    drop(reader);
    child.wait_with_output().expect("wait for actus")
}

fn assert_quiet(output: &Output, context: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "{context}: a closed stdout must print nothing, got: {stderr}");
}

fn write_manifest(project: &Path) {
    fs::create_dir_all(project.join("tests")).expect("create tests directory");
    fs::write(
        project.join("Actus.toml"),
        "[package]\nname = \"closed\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
}

fn test_verbs(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|index| format!("meta test\nverb {prefix}{index:04}() -> Int {{ return 0; }}\n"))
        .collect()
}

#[test]
fn version_stops_quietly_when_stdout_is_closed_first() {
    let sandbox = Sandbox::new("version");
    let mut stopped = 0;
    // The command may win the race against the closing of the pipe and finish;
    // it must never panic either way.
    for _ in 0..10 {
        let output = run_with_closed_reader(&sandbox, &sandbox.root, &["--version"], 0);
        assert_quiet(&output, "--version");
        match output.status.code() {
            Some(0) => {}
            Some(CLOSED_STREAM_STATUS) => stopped += 1,
            other => panic!("unexpected status {other:?}"),
        }
    }
    assert!(stopped > 0, "the closed pipe never stopped `actus --version`");
}

#[test]
fn test_list_stops_quietly_when_stdout_is_closed_midway() {
    let sandbox = Sandbox::new("list");
    let project = sandbox.project();
    write_manifest(&project);
    // More than any pipe buffer holds, so that the command is still writing
    // when the reader goes away.
    let names = "a".repeat(120);
    fs::write(project.join("tests").join("many.act"), test_verbs(&names, 900))
        .expect("write tests");
    let output = run_with_closed_reader(&sandbox, &project, &["test", "--list"], 1);
    assert_quiet(&output, "test --list");
    assert_eq!(output.status.code(), Some(CLOSED_STREAM_STATUS));
    assert!(sandbox.leftovers().is_empty(), "left behind: {:?}", sandbox.leftovers());
}

// Building and linking a test executable is exercised on Unix hosts only, like
// the rest of the `actus test` workflow tests.
#[cfg(unix)]
#[test]
fn test_run_stops_quietly_and_removes_its_artifacts_when_stdout_is_closed() {
    let sandbox = Sandbox::new("run");
    let project = sandbox.project();
    write_manifest(&project);
    fs::write(project.join("tests").join("a_first.act"), test_verbs("first", 12))
        .expect("write first tests");
    fs::write(project.join("tests").join("b_second.act"), test_verbs("second", 3))
        .expect("write second tests");
    let output = run_with_closed_reader(&sandbox, &project, &["test", "--test-threads", "1"], 1);
    assert_quiet(&output, "test");
    assert_eq!(output.status.code(), Some(CLOSED_STREAM_STATUS));
    assert!(sandbox.leftovers().is_empty(), "left behind: {:?}", sandbox.leftovers());
}

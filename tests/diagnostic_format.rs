//! One printed format for every diagnostic (bug #64): `<path>: error[E####] at L:C: message`
//! with a file, `error[E####]: message` without one, and a finding that fails the run is an
//! error, never a warning.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Project {
    root: PathBuf,
}

impl Project {
    fn new(name: &str, manifest: &str) -> Self {
        let root = std::env::temp_dir().join(format!("actus-format-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("create project");
        fs::write(root.join("Actus.toml"), manifest).expect("write manifest");
        fs::write(root.join("src/main.act"), "\"\"\"Entry point of the diagnostic-format fixture; it does nothing.\"\"\"\nverb main() {\n}\n").expect("write source");
        Self { root }
    }

    fn run(&self, arguments: &[&str], linker: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_actus"));
        command.args(arguments).current_dir(&self.root).env_remove("ACTUS_LINKER");
        if let Some(linker) = linker {
            command.env("ACTUS_LINKER", linker);
        }
        command.output().expect("run actus")
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn first_stderr_line(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).lines().next().unwrap_or_default().to_owned()
}

/// Splits `<path>: error[...` into the path and the rest starting at `error[`.
fn split_located(line: &str) -> (&Path, &str) {
    let (path, rest) = line.split_once(": error[").expect("a located error line");
    (Path::new(path), rest)
}

const INVALID_NAME: &str = "[package]\nname = \"1app\"\nversion = \"1.0.0\"\n";

#[test]
fn a_manifest_error_names_the_manifest_the_code_and_the_value() {
    let project = Project::new("manifest-error", INVALID_NAME);
    let output = project.run(&["check"], None);
    assert!(!output.status.success());
    let line = first_stderr_line(&output);
    let (path, rest) = split_located(&line);
    assert_eq!(path.file_name().and_then(|name| name.to_str()), Some("Actus.toml"));
    assert_eq!(
        rest,
        "E1801] at 2:8: Actus.toml package name `1app` is not a valid identity namespace"
    );
}

#[test]
fn a_manifest_error_has_the_same_code_with_and_without_strict() {
    let project = Project::new("manifest-code", INVALID_NAME);
    let plain = first_stderr_line(&project.run(&["check"], None));
    let strict = first_stderr_line(&project.run(&["check", "--strict"], None));
    assert!(plain.contains(": error[E1801] at 2:8: "), "{plain}");
    assert_eq!(plain, strict);
}

#[test]
fn a_malformed_manifest_is_located_at_the_parser_error() {
    let project = Project::new("manifest-parse", "[package\nname");
    let line = first_stderr_line(&project.run(&["check"], None));
    let (_, rest) = split_located(&line);
    assert!(rest.starts_with("E1800] at 1:9: "), "{line}");
}

const FREESTANDING: &str = "[package]\nname = \"bare\"\nversion = \"1.0.0\"\n\n[build]\ntarget = \"thumbv7em-none-eabi\"\n";

#[test]
fn a_diagnostic_without_a_file_prints_the_code_and_no_location() {
    let project = Project::new("no-file", FREESTANDING);
    let output = project.run(&["check", "--strict"], Some("custom-ld"));
    assert_eq!(output.status.code(), Some(1));
    let line = first_stderr_line(&output);
    assert!(
        line.starts_with("error[E1882]: manually selected freestanding linker `custom-ld`"),
        "{line}"
    );
}

#[test]
fn a_manifest_key_that_selects_the_linker_is_located() {
    let manifest = format!("{FREESTANDING}linker = \"custom-ld\"\n");
    let project = Project::new("linker-key", &manifest);
    let line = first_stderr_line(&project.run(&["check", "--strict"], None));
    let (path, rest) = split_located(&line);
    assert_eq!(path.file_name().and_then(|name| name.to_str()), Some("Actus.toml"));
    assert!(rest.starts_with("E1882] at 7:10: manually selected freestanding linker"), "{line}");
}

fn write_oversized_source(project: &Project) {
    let mut source = (0..501)
        .map(|index| format!("open const FILLER_{index}: Int = {index};\n"))
        .collect::<String>();
    source.push_str("\"\"\"Entry point of the oversized-source fixture; it does nothing.\"\"\"\nverb main() {\n}\n");
    fs::write(project.root.join("src/main.act"), source).expect("write oversized source");
}

#[test]
fn a_strict_finding_that_fails_the_run_is_printed_as_an_error() {
    let project = Project::new("strict-error", "[package]\nname = \"big\"\nversion = \"1.0.0\"\n");
    write_oversized_source(&project);
    let output = project.run(&["check", "--strict"], None);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains(": error[E1850] at 1:1: "), "{stderr}");
    assert!(!stderr.contains("warning["), "{stderr}");
}

#[test]
fn a_warning_does_not_fail_the_run_and_the_same_finding_is_an_error_in_strict_mode() {
    let project = Project::new("warning-rule", "[package]\nname = \"big\"\nversion = \"1.0.0\"\n");
    write_oversized_source(&project);
    // `actus conformance` scans the source roots of the repository layout.
    for directory in ["library/std/src", "library/std/tests", "tests"] {
        fs::create_dir_all(project.root.join(directory)).expect("create source root");
    }
    fs::create_dir_all(project.root.join("docs/conformance")).expect("create docs");
    fs::write(project.root.join("docs/conformance/source-exceptions.toml"), "version = 1\n")
        .expect("write the source-exception manifest");
    let plain = project.run(&["conformance"], None);
    let plain_text = String::from_utf8_lossy(&plain.stderr);
    assert_eq!(plain.status.code(), Some(0), "{plain_text}");
    assert!(plain_text.contains(": warning[E1850] at 1:1: "), "{plain_text}");
    let strict = project.run(&["conformance", "--strict"], None);
    let strict_text = String::from_utf8_lossy(&strict.stderr);
    assert_eq!(strict.status.code(), Some(1), "{strict_text}");
    assert!(strict_text.contains(": error[E1850] at 1:1: "), "{strict_text}");
    assert!(!strict_text.contains("warning["), "{strict_text}");
}

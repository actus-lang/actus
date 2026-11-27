//! `meta stack_limit` on a target the backend cannot prove (E1863) is a
//! diagnostic in the unified format with a real location, and `actus check`
//! rejects a program that `actus build` cannot build for its target.

#![cfg(feature = "llvm")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const UNSUPPORTED_TARGET: &str = "aarch64-unknown-linux-gnu";
const SUPPORTED_TARGET: &str = "x86_64-unknown-linux-gnu";

struct Project {
    root: PathBuf,
}

impl Project {
    fn new(name: &str, target: &str) -> Self {
        let root = std::env::temp_dir()
            .join(format!("actus-stack-limit-target-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("create the project directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\ntarget = \"{target}\"\nbackend = \"llvm\"\nruntime = \"std\"\n"
            ),
        )
        .expect("write the manifest");
        Self { root }
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("a file has a parent")).expect("create directory");
        fs::write(path, text).expect("write the source");
    }

    /// Exit code and the stderr of `actus <command>`.
    fn run(&self, command: &str) -> (Option<i32>, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_actus"))
            .arg(command)
            .current_dir(&self.root)
            .output()
            .expect("run actus");
        (output.status.code(), String::from_utf8_lossy(&output.stderr).replace('\\', "/"))
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const STACK_LIMITED_MAIN: &str =
    "verb helper() {\n}\n\nmeta stack_limit(bytes: 256u32)\nverb main() {\n}\n";

fn assert_unified_e1863(stderr: &str, file: &str, position: &str, target: &str) {
    let expected = format!(
        "{file}: error[E1863] at {position}: stack_limit proof is not supported by backend `llvm` for target `{target}`"
    );
    let reported = stderr.lines().any(|line| line.ends_with(&expected));
    assert!(reported, "expected a line ending with `{expected}` in:\n{stderr}");
    assert!(!stderr.contains("SemanticError"), "Debug output reached the user:\n{stderr}");
    assert!(!stderr.contains("SourceSpan"), "Debug output reached the user:\n{stderr}");
}

#[test]
fn check_reports_e1863_with_the_location_of_the_declaration() {
    let project = Project::new("check", UNSUPPORTED_TARGET);
    project.write("src/main.act", STACK_LIMITED_MAIN);
    let (code, stderr) = project.run("check");
    assert_eq!(code, Some(1), "{stderr}");
    assert_unified_e1863(&stderr, "src/main.act", "5:1", UNSUPPORTED_TARGET);
}

#[test]
fn build_and_run_report_e1863_in_the_unified_format() {
    let project = Project::new("build", UNSUPPORTED_TARGET);
    project.write("src/main.act", STACK_LIMITED_MAIN);
    for command in ["build", "run"] {
        let (code, stderr) = project.run(command);
        assert_eq!(code, Some(1), "{command}: {stderr}");
        assert_unified_e1863(&stderr, "src/main.act", "5:1", UNSUPPORTED_TARGET);
    }
}

#[test]
fn e1863_in_an_imported_module_points_into_that_module() {
    let project = Project::new("import", UNSUPPORTED_TARGET);
    project.write(
        "src/main.act",
        "import shared;\n\nverb main() -> Int {\n    return shared::load();\n}\n",
    );
    project.write("src/shared/shared.act", "open api;\n");
    project.write(
        "src/shared/api.act",
        "meta stack_limit(bytes: 128u32)\nopen verb load() -> Int {\n    return 7;\n}\n",
    );
    let (code, stderr) = project.run("check");
    assert_eq!(code, Some(1), "{stderr}");
    assert_unified_e1863(&stderr, "src/shared/api.act", "2:6", UNSUPPORTED_TARGET);
}

#[test]
fn a_supported_target_still_checks_and_builds_a_stack_limited_program() {
    let project = Project::new("supported", SUPPORTED_TARGET);
    project.write("src/main.act", STACK_LIMITED_MAIN);
    for command in ["check", "build"] {
        let (code, stderr) = project.run(command);
        assert_eq!(code, Some(0), "{command}: {stderr}");
    }
}

#[test]
fn an_unsupported_capability_is_reported_by_build_with_its_location() {
    let project = Project::new("capability", SUPPORTED_TARGET);
    project.write(
        "src/main.act",
        "meta capability(\"windows.create_new\")\nverb helper() {\n}\n\nverb main() {\n}\n",
    );
    for command in ["check", "build"] {
        let (code, stderr) = project.run(command);
        assert_eq!(code, Some(1), "{command}: {stderr}");
        assert!(
            stderr.lines().any(|line| line.ends_with(
                "src/main.act: error[E1132] at 2:1: capability `windows.create_new` is not available for target `x86_64-unknown-linux-gnu`"
            )),
            "{command}: {stderr}"
        );
        assert!(!stderr.contains("SemanticError"), "{command}: {stderr}");
    }
}

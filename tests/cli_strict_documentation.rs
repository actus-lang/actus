//! ADR-0041: strict mode validates the public documentation of every source
//! file of the package, in `check`, `build` and `test`, and reports it in the
//! unified diagnostic format. Without `--strict` the documentation is not
//! checked.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ENTRY: &str = "\"\"\"Entry point of the strict documentation fixture; it returns the exit status 0.\"\"\"\nverb main() -> Int { return 0; }\n";

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-strict-docs-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"strict-docs\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write lockfile");
    for (path, source) in files {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create directory");
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

/// `check --strict` rejects `source` with `line` as its first diagnostic.
fn assert_rejected(name: &str, source: &str, line: &str) {
    let root = project(name, &[("main.act", &format!("{source}{ENTRY}"))]);
    let output = actus(&root, &["check", "--strict", "src/main.act"]);
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\\', "/");
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    let first = stderr.lines().next().unwrap_or_default();
    assert!(first.starts_with("src/main.act: error[") && first.contains(line), "stderr: {stderr}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn strict_check_reports_a_missing_docstring_as_e1840() {
    assert_rejected(
        "missing",
        "open verb answer() -> Int { return 42; }\n",
        "E1840] at 1:6: verb answer: missing or empty block docstring",
    );
}

#[test]
fn strict_check_reports_an_ownership_contradiction_as_e1844() {
    assert_rejected(
        "contradiction",
        "\"\"\"Return the constant 42; the verb takes ownership of the borrowed value.\"\"\"\nopen verb answer(abs value: Int) -> Int { return 42; }\n",
        "E1844] at 2:6: verb answer: documentation contradicts the declaration ownership contract",
    );
}

#[test]
fn strict_check_reports_an_impossible_result_promise_as_e1845() {
    assert_rejected(
        "misrepresentation",
        "\"\"\"Return the count in Ok and a typed error in Err; the verb never fails.\"\"\"\nopen verb count() -> Result[Int, Int] { return Ok(7); }\n",
        "E1845] at 2:6: verb count: documentation promises behavior that the declared result contract cannot guarantee",
    );
}

#[test]
fn strict_check_validates_the_module_files_and_not_only_the_entry() {
    let root = project(
        "module",
        &[
            (
                "main.act",
                "import { open shapes };\n\"\"\"Entry point of the strict documentation fixture; it returns the exit status 0.\"\"\"\nverb main() -> Int { return area(); }\n",
            ),
            (
                "shapes/shapes.act",
                "\"\"\"Re-export the area declarations of the shapes module.\"\"\"\nopen area;\n",
            ),
            ("shapes/area.act", "open verb area() -> Int { return 0; }\n"),
        ],
    );
    let output = actus(&root, &["check", "--strict", "src/main.act"]);
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\\', "/");
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("src/shapes/area.act: error[E1840] at 1:6: verb area: missing"),
        "stderr: {stderr}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn strict_build_and_test_reject_an_undocumented_public_verb() {
    let root = project(
        "build-test",
        &[("main.act", &format!("open verb answer() -> Int {{ return 42; }}\n{ENTRY}"))],
    );
    let build = actus(&root, &["build", "--strict", "--emit", "obj", "-o", "out.o"]);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert_eq!(build.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("error[E1840] at 1:6: verb answer"), "stderr: {stderr}");
    assert!(!root.join("out.o").exists());
    let test = actus(&root, &["test", "--strict"]);
    let stderr = String::from_utf8_lossy(&test.stderr);
    assert_eq!(test.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("error[E1840] at 1:6: verb answer"), "stderr: {stderr}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_without_strict_does_not_validate_documentation() {
    let root = project(
        "standard",
        &[("main.act", &format!("open verb answer() -> Int {{ return 42; }}\n{ENTRY}"))],
    );
    let output = actus(&root, &["check", "src/main.act"]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let strict = actus(&root, &["check", "--strict", "src/main.act"]);
    assert_eq!(strict.status.code(), Some(1));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn strict_check_accepts_documented_modules() {
    let root = project(
        "documented",
        &[
            (
                "main.act",
                "import { open shapes };\n\"\"\"Entry point of the strict documentation fixture; it returns the exit status 0.\"\"\"\nverb main() -> Int { return area(); }\n",
            ),
            (
                "shapes/shapes.act",
                "\"\"\"Re-export the area declarations of the shapes module.\"\"\"\nopen area;\n",
            ),
            (
                "shapes/area.act",
                "\"\"\"Return the constant area 0 as an integer value.\"\"\"\nopen verb area() -> Int { return 0; }\n",
            ),
        ],
    );
    let output = actus(&root, &["check", "--strict", "src/main.act"]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn strict_check_does_not_enforce_the_content_contract_yet() {
    // E1841 and E1842 are redesigned in phase 56 goal 5; a documented verb whose
    // docstring has no contract words passes.
    let root = project(
        "content",
        &[(
            "main.act",
            "\"\"\"Compute the doubled number for the caller.\"\"\"\nopen verb double(erg x: Int) -> Int { return x + x; }\n\"\"\"Return the answer.\"\"\"\nopen verb answer() -> Int { return 42; }\n\"\"\"Entry point of the strict documentation fixture; it returns the exit status 0.\"\"\"\nverb main() -> Int { return 0; }\n",
        )],
    );
    let output = actus(&root, &["check", "--strict", "src/main.act"]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let _ = fs::remove_dir_all(root);
}

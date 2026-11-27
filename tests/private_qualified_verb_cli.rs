//! A verb that is not `open` in a module is reported as private (E1109) when
//! another module calls it qualified, instead of as an unknown verb (E1069).
//! `check` and `build` agree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FACADE: &str =
    "\"\"\"Re-export the api module declarations for the fixture project.\"\"\" open api;\n";
const API: &str = "\"\"\"Returns the constant 1.\"\"\"\nverb helper() -> Int { return 1; }\n\"\"\"Return the fixed integer value 2 for the qualified call fixture.\"\"\" open verb shown() -> Int { return 2; }\nconst K: Int = 3;\n";

fn project(name: &str, main: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-private-qualified-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/m")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"private-qualified\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write lockfile");
    fs::write(root.join("src/main.act"), main).expect("write main");
    fs::write(root.join("src/m/m.act"), FACADE).expect("write facade");
    fs::write(root.join("src/m/api.act"), API).expect("write api");
    root
}

fn actus(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("run actus")
}

/// Runs `check` and `build`, returning each command's success and stderr.
fn both(root: &Path) -> Vec<(bool, String)> {
    [
        vec!["check", "--strict", "src/main.act"],
        vec!["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"],
    ]
    .iter()
    .map(|command| {
        let output = actus(root, command);
        (output.status.success(), String::from_utf8_lossy(&output.stderr).into_owned())
    })
    .collect()
}

#[test]
fn private_verb_called_qualified_is_private_not_unknown() {
    let root = project(
        "verb",
        "import m; \"\"\"Entry point that returns the result of helper in module m.\"\"\"\nverb main() -> Int {\n    return m::helper();\n}\n",
    );
    for (success, stderr) in both(&root) {
        assert!(!success, "{stderr}");
        assert!(stderr.contains("error[E1109] at 3:12:"), "{stderr}");
        assert!(stderr.contains("private declaration `helper` from module `m`"), "{stderr}");
        assert!(stderr.contains("mark `helper` as `open`"), "{stderr}");
        assert!(!stderr.contains("E1069") && !stderr.contains("unknown verb"), "{stderr}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn private_constant_read_qualified_is_private_not_undeclared() {
    let root = project(
        "constant",
        "import m; \"\"\"Entry point that returns the constant K of module m.\"\"\"\nverb main() -> Int {\n    return m::K;\n}\n",
    );
    for (success, stderr) in both(&root) {
        assert!(!success, "{stderr}");
        assert!(stderr.contains("error[E1109] at 3:12:"), "{stderr}");
        assert!(stderr.contains("private declaration `K` from module `m`"), "{stderr}");
        assert!(!stderr.contains("E1003"), "{stderr}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn nonexistent_qualified_verb_is_still_unknown() {
    let root = project(
        "missing",
        "import m; \"\"\"Entry point that calls the verb nothing of module m, which is declared without exposing it.\"\"\"\nverb main() -> Int {\n    return m::nothing();\n}\n",
    );
    for (success, stderr) in both(&root) {
        assert!(!success, "{stderr}");
        assert!(stderr.contains("error[E1069] at 3:12:"), "{stderr}");
        assert!(stderr.contains("unknown verb `m::nothing`"), "{stderr}");
        assert!(!stderr.contains("E1109"), "{stderr}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn open_verb_called_qualified_builds() {
    let root = project(
        "open",
        "import m; \"\"\"Entry point that returns the result of shown in module m minus 2.\"\"\"\nverb main() -> Int {\n    return m::shown() - 2;\n}\n",
    );
    for (success, stderr) in both(&root) {
        assert!(success, "{stderr}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Only the importer's own namespace is stripped: `other::helper` names no
/// imported module, so it stays an unknown verb even though `m` has a private
/// `helper`. (Two modules cannot both declare `helper`, E1110, so a
/// same-name open verb in a second module is not expressible.)
#[test]
fn private_name_behind_an_unrelated_qualifier_is_still_unknown() {
    let root = project(
        "unrelated",
        "import m; \"\"\"Entry point that calls helper in the module other, which the test expects to be unreachable from here.\"\"\"\nverb main() -> Int {\n    return other::helper();\n}\n",
    );
    for (success, stderr) in both(&root) {
        assert!(!success, "{stderr}");
        assert!(stderr.contains("error[E1069] at 3:12:"), "{stderr}");
        assert!(stderr.contains("unknown verb `other::helper`"), "{stderr}");
        assert!(!stderr.contains("E1109"), "{stderr}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

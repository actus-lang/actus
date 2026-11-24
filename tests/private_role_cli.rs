//! A role, struct or enum that is not `open` in a module is reported as private
//! (E1109) wherever another module names it in a signature, binding, bound or
//! `perform`, instead of as an unknown type or role (E1023, E1056, E1099).
//! `check` and `build` agree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FACADE: &str = "open api;\n";
const API: &str = "struct File { value: Int, }
role R { verb write(abs self: File) -> Int; }
perform R for File { verb write(abs self: File) -> Int { return self.value; } }
enum E { A, B, }
open verb shown() -> Int { return 2; }
";
const OPEN_API: &str = "open struct File { value: Int, }
open role R { verb write(abs self: File) -> Int; }
open perform R for File { verb write(abs self: File) -> Int { return self.value; } }
open verb shown() -> Int { return 2; }
";
const MAIN_TAIL: &str = "verb main() -> Int { return 0; }\n";

fn project(name: &str, api: &str, main: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-private-role-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/m")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"private-role\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write lockfile");
    fs::write(root.join("src/main.act"), format!("import {{ open m }};\n{main}{MAIN_TAIL}"))
        .expect("write main");
    fs::write(root.join("src/m/m.act"), FACADE).expect("write facade");
    fs::write(root.join("src/m/api.act"), api).expect("write api");
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

/// `main` (after the import line) must be rejected as private `symbol` at `position`.
fn assert_private(name: &str, main: &str, symbol: &str, position: &str) {
    let root = project(name, API, main);
    for (success, stderr) in both(&root) {
        assert!(!success, "{stderr}");
        assert!(stderr.contains(&format!("error[E1109] at {position}:")), "{stderr}");
        assert!(
            stderr.contains(&format!("private declaration `{symbol}` from module `m`")),
            "{stderr}"
        );
        assert!(stderr.contains(&format!("mark `{symbol}` as `open`")), "{stderr}");
        for code in ["E1023", "E1056", "E1099"] {
            assert!(!stderr.contains(code), "{stderr}");
        }
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn private_role_as_dynamic_parameter_is_private() {
    assert_private("dynamic", "verb f(abs w: dynamic R) -> Int { return 0; }\n", "R", "2:23");
}

#[test]
fn private_role_as_parameter_type_is_private() {
    assert_private("param", "verb f(abs w: R) -> Int { return 0; }\n", "R", "2:15");
}

#[test]
fn private_role_as_return_type_is_private() {
    assert_private("return", "verb f() -> R { return 0; }\n", "R", "2:13");
}

#[test]
fn private_role_as_binding_type_is_private() {
    assert_private("binding", "verb h() -> Int { erg x: R = 1; return 0; }\n", "R", "2:19");
}

#[test]
fn private_role_as_struct_field_type_is_private() {
    assert_private("field", "struct S { w: R, }\n", "R", "2:15");
}

#[test]
fn private_role_as_generic_bound_is_private() {
    assert_private("bound", "verb g[T: R](abs i: T) -> Int { return 0; }\n", "R", "2:11");
}

#[test]
fn private_role_in_perform_is_private() {
    let main = "struct Mine { v: Int, }\nperform R for Mine { verb write(abs self: Mine) -> Int { return 0; } }\n";
    assert_private("perform", main, "R", "3:1");
}

#[test]
fn private_struct_as_parameter_type_is_private() {
    assert_private("struct-param", "verb f(abs w: File) -> Int { return 0; }\n", "File", "2:15");
}

#[test]
fn private_enum_as_parameter_type_is_private() {
    assert_private("enum-param", "verb f(abs w: E) -> Int { return 0; }\n", "E", "2:15");
}

#[test]
fn private_struct_in_enum_tuple_payload_is_private() {
    assert_private("enum-tuple", "enum Wrap { Full(File), Empty, }\n", "File", "2:18");
}

#[test]
fn private_enum_in_enum_struct_payload_is_private() {
    assert_private("enum-struct", "enum Wrap { Full { v: Array[E, 2] }, Empty, }\n", "E", "2:29");
}

#[test]
fn open_role_in_every_position_builds() {
    let main = "verb f(abs w: dynamic R) -> Int { return 0; }
verb g[T: R](abs i: T) -> Int { return 0; }
verb main2() -> Int { erg file = File { value: 1, }; return f(w: ref file) + shown() - 2; }
";
    let root = project("open", OPEN_API, main);
    for (success, stderr) in both(&root) {
        assert!(success, "{stderr}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

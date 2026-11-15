//! ADR-0112: symbol linkage follows declaration reach.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, sources: &[(&str, &str)]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-symbol-linkage-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create symbol linkage fixture");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"symbol-linkage-{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n"
        ),
    )
    .expect("write symbol linkage manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write symbol linkage lockfile");
    for (path, source) in sources {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create source directory");
        fs::write(path, source).expect("write symbol linkage source");
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn check_build_run(root: &Path) -> Option<i32> {
    let check = actus(root, &["check", "--strict", "src/main.act"]);
    assert!(check.status.success(), "check stderr: {}", stderr(&check));
    let build =
        actus(root, &["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"]);
    assert!(build.status.success(), "build stderr: {}", stderr(&build));
    Command::new(root.join("main.bin")).output().expect("run built executable").status.code()
}

/// Symbols of the unit object `pkg`: name to "is global".
fn unit_symbols(root: &Path) -> std::collections::BTreeMap<String, bool> {
    use object::{Object, ObjectSymbol};
    let directory = root.join("capsula/debug");
    let target = fs::read_dir(&directory)
        .expect("read capsula")
        .next()
        .expect("one target directory")
        .expect("target entry")
        .path();
    let object_path = fs::read_dir(&target)
        .expect("read target objects")
        .map(|entry| entry.expect("object entry").path())
        .find(|path| path.to_string_lossy().ends_with("actus_mod_3_pkg.obj"))
        .expect("unit object for `pkg`");
    let bytes = fs::read(object_path).expect("read unit object");
    let file = object::File::parse(bytes.as_slice()).expect("parse unit object");
    file.symbols()
        .filter(|symbol| symbol.is_definition())
        .filter_map(|symbol| {
            let name = symbol.name().ok()?.to_owned();
            name.contains("_verb_").then_some((name, symbol.is_global()))
        })
        .collect()
}

fn linkage_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "main.act",
            "import {\n    open pkg,\n};\n\nverb main() -> Int {\n    return public_value() - 3;\n}\n",
        ),
        ("pkg/pkg.act", "open api;\n\nopen unit shared;\n"),
        (
            "pkg/api.act",
            "open verb public_value() -> Int {\n    return hidden_value() + shared_value();\n}\n\nverb hidden_value() -> Int {\n    return 1;\n}\n",
        ),
        ("pkg/shared.act", "open verb shared_value() -> Int {\n    return 2;\n}\n"),
    ]
}

fn global(symbols: &std::collections::BTreeMap<String, bool>, verb: &str) -> bool {
    *symbols
        .iter()
        .find(|(name, _)| name.ends_with(&format!("_verb_{verb}")))
        .unwrap_or_else(|| panic!("`{verb}` is defined in the unit object: {symbols:?}"))
        .1
}

/// ADR-0112: only a public verb, the entry, and symbols other objects name
/// stay global; private and unit-scoped verbs are local to their object.
#[test]
fn symbol_linkage_follows_declaration_reach() {
    let root = project("reach", &linkage_sources());
    assert_eq!(check_build_run(&root), Some(0));
    let build = actus(&root, &["build", "--strict", "src/main.act"]);
    assert!(build.status.success(), "build stderr: {}", stderr(&build));
    let symbols = unit_symbols(&root);
    assert!(global(&symbols, "public_5fvalue"), "{symbols:?}");
    assert!(!global(&symbols, "hidden_5fvalue"), "{symbols:?}");
    assert!(!global(&symbols, "shared_5fvalue"), "{symbols:?}");
    fs::remove_dir_all(root).expect("remove reach fixture");
}

/// A test plan calls private verbs across objects, so its symbols stay
/// global and the test links.
#[test]
fn test_plans_link_private_verbs_across_objects() {
    let mut sources = linkage_sources();
    sources.push((
        "../tests/pkg/api.act",
        "meta test\nverb hidden_is_one() -> Int {\n    return hidden_value() - 1;\n}\n",
    ));
    let root = project("test-plan", &sources);
    let output = actus(&root, &["test", "--strict"]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("hidden_is_one ... ok"));
    fs::remove_dir_all(root).expect("remove test-plan fixture");
}

//! ADR-0112 and ADR-0113: one object per program, whose only global
//! definition is the entry.

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

/// Defined symbols of the program object: name to "is global".
fn program_symbols(root: &Path) -> std::collections::BTreeMap<String, bool> {
    use object::{Object, ObjectSymbol};
    let directory = root.join("capsula/debug");
    let target = fs::read_dir(&directory)
        .expect("read capsula")
        .next()
        .expect("one target directory")
        .expect("target entry")
        .path();
    let objects = fs::read_dir(&target)
        .expect("read target objects")
        .map(|entry| entry.expect("object entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "obj"))
        .collect::<Vec<_>>();
    assert_eq!(objects.len(), 1, "one object per program: {objects:?}");
    let bytes = fs::read(&objects[0]).expect("read program object");
    let file = object::File::parse(bytes.as_slice()).expect("parse program object");
    file.symbols()
        .filter(|symbol| symbol.is_definition())
        .filter_map(|symbol| {
            let name = symbol.name().ok()?.to_owned();
            (name.contains("_verb_") || name.trim_start_matches('_') == "main")
                .then_some((name, symbol.is_global()))
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

/// ADR-0113: the entry is the only global definition; public, private and
/// unit-scoped verbs are all local to the program object.
#[test]
fn program_object_keeps_only_the_entry_global() {
    let root = project("reach", &linkage_sources());
    assert_eq!(check_build_run(&root), Some(0));
    let build = actus(&root, &["build", "--strict", "src/main.act"]);
    assert!(build.status.success(), "build stderr: {}", stderr(&build));
    let symbols = program_symbols(&root);
    let global =
        symbols.iter().filter(|(_, global)| **global).map(|(name, _)| name.trim_start_matches('_'));
    assert_eq!(global.collect::<Vec<_>>(), ["main"], "{symbols:?}");
    for verb in ["public_5fvalue", "hidden_5fvalue", "shared_5fvalue"] {
        assert!(
            symbols.keys().any(|name| name.ends_with(&format!("_verb_{verb}"))),
            "`{verb}` is defined in the program object: {symbols:?}"
        );
    }
    fs::remove_dir_all(root).expect("remove reach fixture");
}

/// A test plan calls private verbs of its module inside the one program
/// object, with no linkage exception.
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

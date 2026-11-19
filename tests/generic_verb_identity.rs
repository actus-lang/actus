//! ADR-0108: generic verbs are matched by canonical declaration identity.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, sources: &[(&str, &str)]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-generic-identity-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create generic identity fixture");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"generic-identity-{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n"
        ),
    )
    .expect("write generic identity manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write generic identity lockfile");
    for (path, source) in sources {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create source directory");
        fs::write(path, source).expect("write generic identity source");
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

const MAIN: &str = "import alpha as a;\nimport {\n    open beta,\n};\n\nverb main() -> Int {\n    erg one = 1;\n    erg two = 2;\n    erg three = 3;\n    erg four = 4;\n    erg five = 5;\n    erg six = 6;\n    erg yes = true;\n    erg no = false;\n    erg kept = a::twice(value: erg yes, other: erg no);\n    erg base = a::pick(first: erg one, second: erg two) * 100\n        + choose(first: erg three, second: erg four) * 10;\n    if kept {\n        return base + a::twice(value: erg five, other: erg six);\n    }\n    return 0;\n}\n";

const ALPHA: &str = "open verb pick[T](erg first: T, erg second: T) -> T {\n    return first;\n}\n\nopen verb twice[T](erg value: T, erg other: T) -> T {\n    return pick(first: erg value, second: erg other);\n}\n";

const BETA: &str =
    "open verb choose[T](erg first: T, erg second: T) -> T {\n    return second;\n}\n";

/// A generic verb reached through an aliased import, an open import, and a
/// nested call from another generic verb specializes and links by identity:
/// the open-import binding is not a declaration of the importer.
#[test]
fn generic_verbs_link_through_every_import_binding() {
    let root = project(
        "bindings",
        &[
            ("main.act", MAIN),
            ("alpha/alpha.act", "open api;\n"),
            ("alpha/api.act", ALPHA),
            ("beta/beta.act", "open api;\n"),
            ("beta/api.act", BETA),
        ],
    );
    let check = actus(&root, &["check", "--strict", "src/main.act"]);
    assert!(check.status.success(), "check stderr: {}", stderr(&check));
    let build =
        actus(&root, &["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"]);
    assert!(build.status.success(), "build stderr: {}", stderr(&build));
    let run = Command::new(root.join("main.bin")).output().expect("run built executable");
    assert_eq!(run.status.code(), Some(145));
    fs::remove_dir_all(root).expect("remove generic identity fixture");
}

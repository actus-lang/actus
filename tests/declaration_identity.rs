//! ADR-0108: declarations are identified by package, module path, and name,
//! never by their short spelling.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, sources: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!("actus-declaration-identity-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create declaration identity fixture");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"identity-{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n"
        ),
    )
    .expect("write declaration identity manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write declaration identity lockfile");
    for (path, source) in sources {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create source directory");
        fs::write(path, source).expect("write declaration identity source");
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

/// Checks, builds, and runs `src/main.act`, returning the process exit code.
fn check_build_run(root: &Path) -> Option<i32> {
    let check = actus(root, &["check", "--strict", "src/main.act"]);
    assert!(check.status.success(), "check stderr: {}", stderr(&check));
    let build =
        actus(root, &["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"]);
    assert!(build.status.success(), "build stderr: {}", stderr(&build));
    Command::new(root.join("main.bin")).output().expect("run built executable").status.code()
}

#[test]
fn size_of_generic_parameter_with_an_imported_struct() {
    let root = project(
        "generic-size-imported",
        &[
            (
                "main.act",
                "import { open shapes };\n\
                 \"\"\"Returns the size in bytes of the type argument T, using the size_of intrinsic.\"\"\"\nverb size[T]() -> u64 { return size_of[T](); }\n\
                 \"\"\"Returns the alignment in bytes of the type argument T, using the align_of intrinsic.\"\"\"\nverb alignment[T]() -> u64 { return align_of[T](); }\n\
                 \"\"\"Entry point that returns 0 when the Wide struct has size 24 and alignment 8, and 1 or 2 when the size or the alignment differs.\"\"\"\nverb main() -> Int {\n\
                 if size[Wide]() != 24u64 { return 1; }\n\
                 if alignment[Wide]() != 8u64 { return 2; }\n\
                 return 0;\n}\n",
            ),
            (
                "shapes/shapes.act",
                "\"\"\"Re-export the wide module declarations for the fixture project.\"\"\" open wide;\n",
            ),
            (
                "shapes/wide.act",
                "\"\"\"Fixture struct with three unsigned integer fields used to measure size and alignment.\"\"\" open struct Wide { \"\"\"The owned first value field of the struct.\"\"\" erg a: u64, \"\"\"The owned second value field of the struct.\"\"\" erg b: u32, \"\"\"The owned third value field of the struct.\"\"\" erg c: u64, }\n",
            ),
        ],
    );
    assert_eq!(check_build_run(&root), Some(0));
    fs::remove_dir_all(root).expect("remove generic-size-imported fixture");
}

/// A type that the facade does not export is rejected wherever it is named:
/// as an explicit generic argument, as an intrinsic type argument, and nested
/// inside another type argument. `check` and `build` report the same E1109.
#[test]
fn private_type_arguments_are_rejected_by_check_and_build() {
    for (name, use_site) in [
        (
            "generic",
            "\"\"\"Returns the size in bytes of the type argument T, using the size_of intrinsic.\"\"\"\nverb size[T]() -> u64 { return size_of[T](); }\n\"\"\"Entry point that returns 0 when the size of the Hidden struct, read through the generic size verb, is 8.\"\"\"\nverb main() -> Int { if size[Hidden]() != 8u64 { return 1; } return 0; }\n",
        ),
        (
            "intrinsic",
            "\"\"\"Entry point that returns 0 when the size_of intrinsic reports 8 for the Hidden struct.\"\"\"\nverb main() -> Int { if size_of[Hidden]() != 8u64 { return 1; } return 0; }\n",
        ),
        (
            "nested",
            "\"\"\"Entry point that returns 0 when a two-element array of Hidden structs has size 16.\"\"\"\nverb main() -> Int { if size_of[Array[Hidden, 2]]() != 16u64 { return 1; } return 0; }\n",
        ),
    ] {
        let main = format!("import {{ open shapes }};\n{use_site}");
        let root = project(
            &format!("private-argument-{name}"),
            &[
                ("main.act", main.as_str()),
                ("shapes/shapes.act", "open visible;\n"),
                ("shapes/visible.act", "open struct Visible { erg a: u64, }\n"),
                ("shapes/hidden.act", "struct Hidden { erg a: u64, }\n"),
            ],
        );
        for command in [
            vec!["check", "--strict", "src/main.act"],
            vec!["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"],
        ] {
            let output = actus(&root, &command);
            assert!(!output.status.success(), "{name}: {} accepted a private type", command[0]);
            let stderr = stderr(&output);
            assert!(stderr.contains("E1109"), "{name}: {} stderr: {stderr}", command[0]);
            assert!(!stderr.contains("internal compiler error"), "{name}: {stderr}");
        }
        fs::remove_dir_all(root).expect("remove private-argument fixture");
    }
}

#[test]
fn size_of_generic_parameter_with_a_root_struct() {
    let root = project(
        "generic-size-root",
        &[(
            "main.act",
            "struct Wide { erg a: u64, erg b: u32, erg c: u64, }\n\
             \"\"\"Returns the size in bytes of the type argument T, using the size_of intrinsic.\"\"\"\nverb size[T]() -> u64 { return size_of[T](); }\n\
             \"\"\"Returns the alignment in bytes of the type argument T, using the align_of intrinsic.\"\"\"\nverb alignment[T]() -> u64 { return align_of[T](); }\n\
             \"\"\"Entry point that returns 0 when the Wide struct has size 24 and alignment 8, and 1 or 2 when the size or the alignment differs.\"\"\"\nverb main() -> Int {\n\
             if size[Wide]() != 24u64 { return 1; }\n\
             if alignment[Wide]() != 8u64 { return 2; }\n\
             return 0;\n}\n",
        )],
    );
    assert_eq!(check_build_run(&root), Some(0));
    fs::remove_dir_all(root).expect("remove generic-size-root fixture");
}

//! ADR-0110: directory modules see their own declarations, their ancestors'
//! declarations, the exports of the children they open, and their own
//! imports. Imports of the unit's own modules bind to the loaded unit.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, sources: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!("actus-hierarchical-scope-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create hierarchical scope fixture");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"scope-{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n"
        ),
    )
    .expect("write hierarchical scope manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write hierarchical scope lockfile");
    for (path, source) in sources {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create source directory");
        fs::write(path, source).expect("write hierarchical scope source");
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

/// `check` and `build` both reject the fixture with E1119 and `expected`.
fn assert_scope_error(root: &Path, expected: &str) {
    for command in [
        vec!["check", "--strict", "src/main.act"],
        vec!["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"],
    ] {
        let output = actus(root, &command);
        assert!(!output.status.success(), "{} accepted an out-of-scope reference", command[0]);
        let stderr = stderr(&output);
        assert!(stderr.contains("E1119"), "{} stderr: {stderr}", command[0]);
        assert!(stderr.contains(expected), "{} stderr: {stderr}", command[0]);
        assert!(!stderr.contains("internal compiler error"), "{stderr}");
    }
}

const MAIN: &str = "import { open pkg };\n\
     verb main() -> Int {\n\
     erg small: Holder[4] = Holder[4] { values: Array[u32, 4](), };\n\
     return train(holder: ins small) as Int - 4;\n}\n";

const LAYOUT: [(&str, &str); 2] = [
    (
        "pkg/layout/layout.act",
        "\"\"\"Re-export the shape module declarations for the fixture project.\"\"\" open shape;\n",
    ),
    (
        "pkg/layout/shape.act",
        "\"\"\"Fixed-capacity holder of N unsigned values for the scope fixture.\"\"\" open struct Holder[N: Usize] { \"\"\"The owned value array of the holder.\"\"\" erg values: Array[u32, N], }\n",
    ),
];

fn r2_sources(count_imports: &str, helper_imports: &str) -> Vec<(&'static str, String)> {
    vec![
        ("main.act", MAIN.to_owned()),
        ("pkg/pkg.act", "\"\"\"Re-export the layout module declarations for the fixture project.\"\"\" open layout;\n\"\"\"Re-export the runtime module declarations for the fixture project.\"\"\" open runtime;\n".to_owned()),
        (LAYOUT[0].0, LAYOUT[0].1.to_owned()),
        (LAYOUT[1].0, LAYOUT[1].1.to_owned()),
        ("pkg/runtime/runtime.act", "\"\"\"Re-export the count module declarations for the fixture project.\"\"\" open count;\n\"\"\"Re-export the training module declarations for the fixture project.\"\"\" open training;\n".to_owned()),
        (
            "pkg/runtime/count.act",
            format!(
                "{count_imports}\"\"\"Borrow the holder through an exclusive ins loan and return its capacity N as an unsigned value.\"\"\" open verb count[N: Usize](ins holder: Holder[N]) -> u32 {{ return N as u32; }}\n"
            ),
        ),
        ("pkg/runtime/training/training.act", "\"\"\"Re-export the helper module declarations for the fixture project.\"\"\" open helper;\n".to_owned()),
        (
            "pkg/runtime/training/helper.act",
            format!(
                "import {{ pkg::runtime, {helper_imports} }};\n\
                 \"\"\"Loan the holder through ins to the runtime count verb and return the capacity value it reports.\"\"\" open verb train[N: Usize](ins holder: Holder[N]) -> u32 {{ return runtime::count(holder: ins holder); }}\n"
            ),
        ),
    ]
}

fn borrowed<'a>(sources: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    sources.iter().map(|(path, source)| (*path, source.as_str())).collect()
}

/// Phase 53 R2: an import of an ancestor module inside a unit must not load a
/// second, incomplete unit. With the sibling-directory imports that ADR-0110
/// requires, the const-generic struct resolves and the program runs.
#[test]
fn import_of_an_ancestor_module_binds_to_the_loaded_unit() {
    let sources = r2_sources("import { open pkg::layout };\n", "open pkg::layout");
    let root = project("ancestor-import", &borrowed(&sources));
    assert_eq!(check_build_run(&root), Some(0));
    fs::remove_dir_all(root).expect("remove ancestor-import fixture");
}

#[test]
fn sibling_directory_declaration_requires_an_import() {
    let sources = r2_sources("", "open pkg::layout");
    let root = project("sibling-without-import", &borrowed(&sources));
    assert_scope_error(
        &root,
        "`Holder` is declared in module `pkg::layout`, which is not in scope of module `pkg::runtime`",
    );
    fs::remove_dir_all(root).expect("remove sibling-without-import fixture");
}

#[test]
fn imports_are_not_inherited_by_child_modules() {
    let sources = r2_sources("import { open pkg::layout };\n", "");
    let root = project("import-not-inherited", &borrowed(&sources));
    assert_scope_error(&root, "not in scope of module `pkg::runtime::training`");
    fs::remove_dir_all(root).expect("remove import-not-inherited fixture");
}

#[test]
fn child_module_sees_private_ancestor_declarations() {
    let root = project(
        "ancestor-private",
        &[
            ("main.act", "import { open pkg };\nverb main() -> Int { return run() as Int - 7; }\n"),
            (
                "pkg/pkg.act",
                "\"\"\"Re-export the base module declarations for the fixture project.\"\"\" open base;\n\"\"\"Re-export the worker module declarations for the fixture project.\"\"\" open worker;\n",
            ),
            ("pkg/base.act", "verb seven() -> u32 { return 7u32; }\n"),
            (
                "pkg/worker/worker.act",
                "\"\"\"Re-export the job module declarations for the fixture project.\"\"\" open job;\n",
            ),
            (
                "pkg/worker/job.act",
                "\"\"\"Return the unsigned 32-bit value that the private ancestor verb seven reports.\"\"\" open verb run() -> u32 { return seven(); }\n",
            ),
        ],
    );
    assert_eq!(check_build_run(&root), Some(0));
    fs::remove_dir_all(root).expect("remove ancestor-private fixture");
}

#[test]
fn parent_module_cannot_use_a_private_child_declaration() {
    let root = project(
        "private-child",
        &[
            ("main.act", "import { open pkg };\nverb main() -> Int { return run() as Int; }\n"),
            (
                "pkg/pkg.act",
                "\"\"\"Re-export the entry module declarations for the fixture project.\"\"\" open entry;\n\"\"\"Re-export the worker module declarations for the fixture project.\"\"\" open worker;\n",
            ),
            ("pkg/entry.act", "open verb run() -> u32 { return hidden(); }\n"),
            (
                "pkg/worker/worker.act",
                "\"\"\"Re-export the job module declarations for the fixture project.\"\"\" open job;\n",
            ),
            ("pkg/worker/job.act", "verb hidden() -> u32 { return 0u32; }\n"),
        ],
    );
    assert_scope_error(&root, "`hidden` is private to child module `pkg::worker`");
    fs::remove_dir_all(root).expect("remove private-child fixture");
}

#[test]
fn parent_module_uses_the_exports_of_an_opened_child() {
    let root = project(
        "opened-child",
        &[
            ("main.act", "import { open pkg };\nverb main() -> Int { return run() as Int - 3; }\n"),
            (
                "pkg/pkg.act",
                "\"\"\"Re-export the entry module declarations for the fixture project.\"\"\" open entry;\n\"\"\"Re-export the worker module declarations for the fixture project.\"\"\" open worker;\n",
            ),
            (
                "pkg/entry.act",
                "\"\"\"Return the unsigned 32-bit value that the opened child verb three reports.\"\"\" open verb run() -> u32 { return three(); }\n",
            ),
            (
                "pkg/worker/worker.act",
                "\"\"\"Re-export the job module declarations for the fixture project.\"\"\" open job;\n",
            ),
            (
                "pkg/worker/job.act",
                "\"\"\"Return the unsigned 32-bit value 3 for the parent module fixture.\"\"\" open verb three() -> u32 { return 3u32; }\n",
            ),
        ],
    );
    assert_eq!(check_build_run(&root), Some(0));
    fs::remove_dir_all(root).expect("remove opened-child fixture");
}

#[test]
fn qualified_call_through_an_unimported_binding_is_rejected() {
    let root = project(
        "binding-not-inherited",
        &[
            ("main.act", "import { open pkg };\nverb main() -> Int { return run() as Int - 5; }\n"),
            ("pkg/pkg.act", "open entry;\nopen worker;\nopen tools;\n"),
            (
                "pkg/entry.act",
                "import { pkg::tools };\nopen verb run() -> u32 { return tools::five(); }\n",
            ),
            ("pkg/tools/tools.act", "open five;\n"),
            ("pkg/tools/five.act", "open verb five() -> u32 { return 5u32; }\n"),
            (
                "pkg/worker/worker.act",
                "\"\"\"Re-export the job module declarations for the fixture project.\"\"\" open job;\n",
            ),
            ("pkg/worker/job.act", "open verb six() -> u32 { return tools::five() + 1u32; }\n"),
        ],
    );
    assert_scope_error(&root, "imported by module `pkg`, not by `pkg::worker`");
    fs::remove_dir_all(root).expect("remove binding-not-inherited fixture");
}

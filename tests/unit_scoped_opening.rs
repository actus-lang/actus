//! ADR-0111: `open unit x;` opens a sibling or child for its unit only.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, sources: &[(&str, &str)]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-unit-opening-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create unit opening fixture");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"unit-opening-{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n"
        ),
    )
    .expect("write unit opening manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write unit opening lockfile");
    for (path, source) in sources {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create source directory");
        fs::write(path, source).expect("write unit opening source");
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

/// `pkg/store` opens `shard` publicly; `shard` opens its codec for the unit
/// only, and `pkg/store/log` (a cousin of `shard`) uses the codec.
fn unit_sources(main: &str) -> Vec<(&'static str, String)> {
    vec![
        ("main.act", main.to_owned()),
        ("pkg/pkg.act", "open store;\n".to_owned()),
        ("pkg/store/store.act", "open shard;\nopen log;\n".to_owned()),
        ("pkg/store/shard/shard.act", "open fixed;\nopen unit codec;\n".to_owned()),
        ("pkg/store/shard/fixed.act", "open verb fixed_width() -> u32 {\n    return 8u32;\n}\n".to_owned()),
        ("pkg/store/shard/codec.act", "open verb encode_width() -> u32 {\n    return 4u32;\n}\n".to_owned()),
        ("pkg/store/log/log.act", "open entry;\n".to_owned()),
        (
            "pkg/store/log/entry.act",
            "import {\n    open pkg::store::shard,\n};\n\nopen verb entry_width() -> u32 {\n    return encode_width() + fixed_width();\n}\n".to_owned(),
        ),
    ]
}

/// The same sources with the `"""` documentation that strict mode requires for
/// every public declaration. The formatter test keeps the undocumented text.
fn documented(sources: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
    const DOCUMENTATION: [(&str, &str); 9] = [
        ("open store;", "Re-export the store module declarations for the fixture project."),
        ("open shard;", "Re-export the shard module declarations for the fixture project."),
        ("open log;", "Re-export the log module declarations for the fixture project."),
        ("open fixed;", "Re-export the fixed module declarations for the fixture project."),
        ("open unit codec;", "Re-export the codec module declarations for this unit only."),
        ("open entry;", "Re-export the entry module declarations for the fixture project."),
        (
            "open verb fixed_width()",
            "Return the unsigned 32-bit width value 8 that the fixed codec uses.",
        ),
        (
            "open verb encode_width()",
            "Return the unsigned 32-bit width value 4 that the unit codec uses.",
        ),
        (
            "open verb entry_width()",
            "Return the unsigned 32-bit value that sums the encode and fixed widths.",
        ),
    ];
    sources
        .into_iter()
        .map(|(path, source)| {
            let source = DOCUMENTATION.iter().fold(source, |text, (declaration, documentation)| {
                text.replace(declaration, &format!("\"\"\"{documentation}\"\"\" {declaration}"))
            });
            (path, source)
        })
        .collect()
}

fn borrowed<'a>(sources: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    sources.iter().map(|(path, source)| (*path, source.as_str())).collect()
}

const PUBLIC_MAIN: &str = "import {\n    open pkg,\n};\n\n\"\"\"Entry point that returns the entry width as Int minus 12.\"\"\"\nverb main() -> Int {\n    return entry_width() as Int - 12;\n}\n";

#[test]
fn unit_scoped_opening_is_visible_inside_the_unit() {
    let sources = documented(unit_sources(PUBLIC_MAIN));
    let root = project("inside", &borrowed(&sources));
    assert_eq!(check_build_run(&root), Some(0));
    fs::remove_dir_all(root).expect("remove inside fixture");
}

#[test]
fn unit_scoped_opening_is_not_an_export_of_the_unit() {
    let sources = unit_sources(
        "import {\n    open pkg,\n};\n\n\"\"\"Entry point that returns the encode width as Int minus 4.\"\"\"\nverb main() -> Int {\n    return encode_width() as Int - 4;\n}\n",
    );
    let root = project("outside", &borrowed(&sources));
    for command in [
        vec!["check", "--strict", "src/main.act"],
        vec!["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"],
    ] {
        let output = actus(&root, &command);
        assert!(!output.status.success(), "{} accepted a unit-scoped name", command[0]);
        let stderr = stderr(&output);
        assert!(stderr.contains("E1109") && stderr.contains("encode_width"), "{stderr}");
        assert!(!stderr.contains("internal compiler error"), "{stderr}");
    }
    fs::remove_dir_all(root).expect("remove outside fixture");
}

#[test]
fn child_import_from_another_unit_sees_only_public_names() {
    let sources = unit_sources(
        "import {\n    pkg::store::shard,\n};\n\n\"\"\"Entry point that returns the encode width of the shard module as Int minus 4.\"\"\"\nverb main() -> Int {\n    return shard::encode_width() as Int - 4;\n}\n",
    );
    let root = project("child-import", &borrowed(&sources));
    let output = actus(&root, &["check", "--strict", "src/main.act"]);
    let stderr = stderr(&output);
    assert!(!output.status.success() && stderr.contains("E1119"), "{stderr}");
    assert!(stderr.contains("is not exported by module `pkg::store::shard`"), "{stderr}");
    fs::remove_dir_all(root).expect("remove child-import fixture");
}

#[test]
fn opening_one_name_both_ways_is_rejected() {
    let mut sources = unit_sources(PUBLIC_MAIN);
    sources[3].1 = "open fixed;\nopen codec;\nopen unit codec;\n".to_owned();
    let root = project("conflict", &borrowed(&sources));
    let output = actus(&root, &["check", "--strict", "src/main.act"]);
    let stderr = stderr(&output);
    assert!(!output.status.success() && stderr.contains("E1137"), "{stderr}");
    fs::remove_dir_all(root).expect("remove conflict fixture");
}

#[test]
fn a_sibling_named_unit_is_still_opened_by_open_unit() {
    let mut sources = documented(unit_sources(PUBLIC_MAIN));
    sources[3].1 = "\"\"\"Re-export the fixed module declarations for the fixture project.\"\"\" open fixed;\n\"\"\"Re-export the codec module declarations for this unit only.\"\"\" open unit codec;\n\"\"\"Re-export the unit sibling declarations for this unit only.\"\"\" open unit;\n".to_owned();
    sources.push((
        "pkg/store/shard/unit.act",
        "\"\"\"Return the unsigned 32-bit value 0 that the unit sibling reports.\"\"\" open verb unit_width() -> u32 {\n    return 0u32;\n}\n".to_owned(),
    ));
    sources[0].1 = "import {\n    open pkg,\n};\n\n\"\"\"Entry point that returns the sum of the entry width and the unit width as Int minus 12.\"\"\"\nverb main() -> Int {\n    return (entry_width() + unit_width()) as Int - 12;\n}\n".to_owned();
    let root = project("named-unit", &borrowed(&sources));
    assert_eq!(check_build_run(&root), Some(0));
    fs::remove_dir_all(root).expect("remove named-unit fixture");
}

#[test]
fn formatter_preserves_unit_scoped_opening() {
    let sources = unit_sources(PUBLIC_MAIN);
    let root = project("format", &borrowed(&sources));
    let facade = root.join("src/pkg/store/shard/shard.act");
    let output = actus(&root, &["fmt", facade.to_str().expect("utf-8 path")]);
    assert!(output.status.success(), "{}", stderr(&output));
    let formatted = fs::read_to_string(&facade).expect("read formatted facade");
    assert_eq!(formatted, "open fixed;\n\nopen unit codec;\n");
    let output = actus(&root, &["fmt", "--check", facade.to_str().expect("utf-8 path")]);
    assert!(output.status.success(), "{}", stderr(&output));
    fs::remove_dir_all(root).expect("remove format fixture");
}

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn missing_module_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-module-diagnostic-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create module fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"module-diagnostic\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write module fixture manifest");
    fs::write(root.join("src/main.act"), "import missing; verb main() -> Int { return 0; }\n")
        .expect("write missing module source");
    root
}

fn check_missing_module(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", "src/main.act"])
        .current_dir(root)
        .output()
        .expect("run missing module check")
}

fn provenance_project(name: &str, module_source: &str, extra: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-provenance-{name}-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create provenance fixture");
    fs::create_dir_all(root.join("target")).expect("create provenance output");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"provenance\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write provenance manifest");
    fs::write(root.join("src/main.act"), "import module; verb main() -> Int { return 0; }\n")
        .expect("write provenance entry");
    fs::write(root.join("src/module.act"), module_source).expect("write provenance module");
    for (path, source) in extra {
        fs::write(root.join(path), source).expect("write provenance extra source");
    }
    root
}

fn run_provenance(root: &Path, command: &str) -> Output {
    let mut process = Command::new(env!("CARGO_BIN_EXE_actus"));
    process.current_dir(root).args([command, "--strict", "src/main.act"]);
    if command == "build" {
        process.args(["--emit", "obj", "-o", "target/provenance.o"]);
    }
    process.output().expect("run provenance command")
}

fn assert_module_provenance(root: &Path, command: &str, expected_code: &str) {
    let output = run_provenance(root, command);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(expected_code), "stderr was: {stderr}");
    assert!(
        stderr.contains("src/module.act: error[") || stderr.contains("src/module.act: "),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains(" at 1:"), "stderr was: {stderr}");
}

fn duplicate_project() -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-provenance-duplicate-{}", std::process::id()));
    fs::create_dir_all(root.join("src/demo")).expect("create duplicate fixture");
    fs::create_dir_all(root.join("target")).expect("create duplicate output");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"provenance-duplicate\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write duplicate manifest");
    fs::write(root.join("src/main.act"), "import demo; verb main() -> Int { return 0; }\n")
        .expect("write duplicate entry");
    fs::write(root.join("src/demo/demo.act"), "struct Shared { value: Int, }\n")
        .expect("write duplicate facade");
    fs::write(root.join("src/demo/other.act"), "struct Shared { value: Int, }\n")
        .expect("write duplicate sibling");
    root
}

fn duplicate_facade_project() -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!("actus-provenance-identical-facades-{}", std::process::id()));
    fs::create_dir_all(root.join("src/left")).expect("create left facade");
    fs::create_dir_all(root.join("src/right")).expect("create right facade");
    fs::create_dir_all(root.join("target")).expect("create duplicate output");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"provenance-identical-facades\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write duplicate manifest");
    fs::write(
        root.join("src/main.act"),
        "import left; import right; verb main() -> Int { return 0; }\n",
    )
    .expect("write duplicate entry");
    fs::write(root.join("src/left/left.act"), "open api;\n").expect("write left facade");
    fs::write(root.join("src/right/right.act"), "open api;\n").expect("write right facade");
    let api = "open enum SharedError { Missing, } unsafe extern \"C\" verb shared_bridge(abs buffer: Buffer) -> Int;\n";
    fs::write(root.join("src/left/api.act"), api).expect("write left declarations");
    fs::write(root.join("src/right/api.act"), api).expect("write right declarations");
    root
}

#[test]
fn check_and_build_report_module_lexical_provenance() {
    let root = provenance_project("lex", "@\n", &[]);
    assert_module_provenance(&root, "check", "E0001");
    assert_module_provenance(&root, "build", "E0001");
    fs::remove_dir_all(root).expect("remove provenance fixture");
}

#[test]
fn check_and_build_report_module_parse_provenance() {
    let root = provenance_project("parse", "verb broken( {\n", &[]);
    assert_module_provenance(&root, "check", "E000");
    assert_module_provenance(&root, "build", "E000");
    fs::remove_dir_all(root).expect("remove provenance fixture");
}

#[test]
fn check_and_build_preserve_cross_file_duplicate_provenance() {
    let root = duplicate_project();
    for command in ["check", "build"] {
        let output = run_provenance(&root, command);
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E1106"), "stderr was: {stderr}");
        assert!(stderr.contains("src/demo/other.act"), "stderr was: {stderr}");
    }
    fs::remove_dir_all(root).expect("remove duplicate fixture");
}

#[test]
fn check_and_build_accept_identical_declarations_with_distinct_source_spans() {
    let root = duplicate_facade_project();
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", "--strict", "src/main.act"])
        .current_dir(&root)
        .output()
        .expect("run duplicate facade check");
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));

    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--strict", "--emit", "obj", "-o", "target/identical.o"])
        .current_dir(&root)
        .output()
        .expect("run duplicate facade build");
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
    fs::remove_dir_all(root).expect("remove duplicate facade fixture");
}

#[test]
fn missing_module_diagnostics_are_deterministic_and_stable() {
    let root = missing_module_project();
    let first = check_missing_module(&root);
    let second = check_missing_module(&root);

    assert!(!first.status.success());
    assert_eq!(first.status.code(), second.status.code());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert!(String::from_utf8_lossy(&first.stderr).contains("error[E1101]"));
    assert!(String::from_utf8_lossy(&first.stderr).contains("missing its canonical facade"));
    fs::remove_dir_all(root).expect("remove module fixture");
}

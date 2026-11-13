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

fn neutral_reexport_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!("actus-neutral-reexport-{}", std::process::id()));
    for directory in ["src/interface/config", "src/app"] {
        fs::create_dir_all(root.join(directory)).expect("create neutral re-export fixture");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"neutral-reexport\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write neutral re-export manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write neutral re-export lockfile");
    fs::write(
        root.join("src/main.act"),
        "import app;\nverb main() -> Int { return app::start(); }\n",
    )
    .expect("write neutral re-export entry");
    fs::write(root.join("src/app/app.act"), "open start;\n").expect("write neutral app facade");
    fs::write(root.join("src/app/start.act"), "import interface;\nopen verb start() -> Int {\n    erg result: Result[Int, interface::ParseError] = interface::parse();\n    return 0;\n}\n")
        .expect("write neutral start source");
    fs::write(root.join("src/interface/interface.act"), "open config;\nopen parser;\n")
        .expect("write neutral interface facade");
    fs::write(root.join("src/interface/config/config.act"), "open contract;\n")
        .expect("write neutral config facade");
    fs::write(
        root.join("src/interface/config/contract.act"),
        "open enum ParseError { Invalid, }\n",
    )
    .expect("write neutral contract source");
    fs::write(root.join("src/interface/parser.act"), "import interface::config;\nopen verb parse() -> Result[Int, config::ParseError] {\n    return Result[Int, config::ParseError].Ok(0);\n}\n")
        .expect("write neutral parser source");
    root
}

#[test]
// Known defect R1: generic specialization treats one type reached through two re-exports as two instances. PR B changes this test to build and run successfully.
fn reexported_type_identity_passes_validation_and_fails_in_specialization() {
    let root = neutral_reexport_project();
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", "--strict", "src/main.act"])
        .current_dir(&root)
        .output()
        .expect("run neutral re-export check");
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));

    let executable = root.join("target/neutral");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&executable)
        .arg("src/main.act")
        .current_dir(&root)
        .output()
        .expect("run neutral re-export build");
    assert!(!build.status.success());
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("internal compiler error: semantic validation invariant violated"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("E1027"), "stderr was: {stderr}");
    fs::remove_dir_all(root).expect("remove neutral re-export fixture");
}

fn conflicting_reexport_project() -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-conflicting-reexport-{}", std::process::id()));
    for directory in ["src/interface/config", "src/app"] {
        fs::create_dir_all(root.join(directory)).expect("create conflicting re-export fixture");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"conflicting-reexport\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write conflicting re-export manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write conflicting re-export lockfile");
    fs::write(
        root.join("src/main.act"),
        "import app;\nverb main() -> Int { return app::start(); }\n",
    )
    .expect("write conflicting re-export entry");
    fs::write(root.join("src/app/app.act"), "open start;\nopen contract;\n")
        .expect("write conflicting app facade");
    fs::write(
        root.join("src/app/start.act"),
        "import interface;\nopen verb start() -> Int {\n    erg result: Result[Int, app::AppParseError] = interface::parse();\n    return 0;\n}\n",
    )
    .expect("write conflicting start source");
    fs::write(root.join("src/app/contract.act"), "open enum AppParseError { Missing, }\n")
        .expect("write conflicting app contract");
    fs::write(root.join("src/interface/interface.act"), "open config;\nopen parser;\n")
        .expect("write conflicting interface facade");
    fs::write(root.join("src/interface/config/config.act"), "open contract;\n")
        .expect("write conflicting config facade");
    fs::write(
        root.join("src/interface/config/contract.act"),
        "open enum ParseError { Invalid, }\n",
    )
    .expect("write conflicting interface contract");
    fs::write(
        root.join("src/interface/parser.act"),
        "import interface::config;\nopen verb parse() -> Result[Int, config::ParseError] {\n    return Result[Int, config::ParseError].Ok(0);\n}\n",
    )
    .expect("write conflicting parser source");
    root
}

#[test]
fn check_and_build_reject_different_reexported_type_identities() {
    let root = conflicting_reexport_project();
    for command in ["check", "build"] {
        let output = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args([command, "--strict", "src/main.act"])
            .current_dir(&root)
            .output()
            .expect("run conflicting re-export command");
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E1027"), "stderr was: {stderr}");
        assert!(stderr.contains("src/app/start.act: error"), "stderr was: {stderr}");
    }
    fs::remove_dir_all(root).expect("remove conflicting re-export fixture");
}

#[test]
fn check_and_build_have_matching_exit_codes_for_example_projects() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut inputs = fs::read_dir(root.join("examples"))
        .expect("read examples")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path().join("Actus.toml");
            path.is_file().then(|| path.parent().expect("manifest parent").to_path_buf())
        })
        .collect::<Vec<_>>();
    inputs.sort();
    inputs.push(root.join("examples/hello.act"));

    for input in inputs {
        let project_root = if input.is_dir() { input.clone() } else { root.clone() };
        let check = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["check", "--strict"])
            .arg(input.file_name().filter(|_| input.is_file()).unwrap_or_default())
            .current_dir(&project_root)
            .output()
            .expect("run example check");
        let output = std::env::temp_dir().join(format!(
            "actus-example-validation-{}-{}.o",
            std::process::id(),
            input.file_stem().and_then(|name| name.to_str()).unwrap_or("project")
        ));
        let build = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["build", "--strict", "--emit", "obj", "-o"])
            .arg(&output)
            .arg(input.file_name().filter(|_| input.is_file()).unwrap_or_default())
            .current_dir(&project_root)
            .output()
            .expect("run example build");
        assert_eq!(
            check.status.code(),
            build.status.code(),
            "check/build status mismatch for {}\ncheck stderr: {}\nbuild stderr: {}",
            input.display(),
            String::from_utf8_lossy(&check.stderr),
            String::from_utf8_lossy(&build.stderr)
        );
        let _ = fs::remove_file(output);
    }
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

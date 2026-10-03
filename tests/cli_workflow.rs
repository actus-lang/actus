use std::fs;
use std::path::Path;
use std::process::Command;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

use actus::cli::run_with_args;

#[test]
fn new_creates_an_actus_project_without_git_when_requested() {
    let root = std::env::temp_dir().join(format!("actus-new-{}", std::process::id()));
    let result = run_with_args(
        vec!["new".to_owned(), root.display().to_string(), "--no-git".to_owned()].into_iter(),
    );

    assert_eq!(result, 0);
    assert!(root.join("Actus.toml").is_file());
    assert!(root.join("src/main.act").is_file());
    assert_eq!(
        fs::read_to_string(root.join(".gitignore")).unwrap(),
        "/capsula/\nActus.lock\n*.o\n*.bin\n*.actus\n"
    );
    assert!(!root.join(".git").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn new_initializes_main_and_prints_next_steps() {
    let root = std::env::temp_dir().join(format!("actus-new-main-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["new", root.to_str().unwrap()])
        .output()
        .expect("run new command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("created `"));
    assert!(stdout.contains("Next steps:"));
    assert!(stdout.contains("actus check"));
    assert!(stdout.contains("actus build --release --emit exe"));
    assert_main_branch_or_parent_repository(&root);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn new_can_select_the_builtin_standard_runtime_profile() {
    let root = std::env::temp_dir().join(format!("actus-new-std-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["new", root.to_str().unwrap(), "--runtime", "std", "--no-git"])
        .output()
        .expect("run new command");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let manifest = fs::read_to_string(root.join("Actus.toml")).expect("read manifest");
    assert!(manifest.contains("runtime = \"std\""));
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("check")
        .current_dir(&root)
        .output()
        .expect("check generated std project");
    assert!(check.status.success(), "stderr: {}", String::from_utf8_lossy(&check.stderr));
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "obj"])
        .current_dir(&root)
        .output()
        .expect("build generated std project");
    assert!(build.status.success(), "stderr: {}", String::from_utf8_lossy(&build.stderr));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_zero_float_policy_accepts_integer_builds() {
    let root = std::env::temp_dir().join(format!("actus-no-float-integer-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"integer_app\"\nversion = \"0.1.0\"\n\n[build]\nverify_no_float_ir = true\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main() -> Int { return 0; }\n")
        .expect("write integer source");
    fs::create_dir_all(root.join("tests")).expect("create test directory");
    fs::write(root.join("tests/smoke.act"), "meta test\nverb smoke() -> Int { return 0; }\n")
        .expect("write integer test");

    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", "--strict"])
        .current_dir(&root)
        .output()
        .expect("run strict integer check");
    assert!(check.status.success(), "stderr: {}", String::from_utf8_lossy(&check.stderr));
    let tests = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["test", "--strict"])
        .current_dir(&root)
        .output()
        .expect("run strict integer tests");
    assert!(tests.status.success(), "stderr: {}", String::from_utf8_lossy(&tests.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "obj"])
        .current_dir(&root)
        .output()
        .expect("run integer zero-float build");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("no floating-point instructions"));
    let executable = root.join("integer_app");
    let executable_build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o", executable.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .expect("build integer executable");
    assert!(
        executable_build.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&executable_build.stderr)
    );
    let run = Command::new(&executable).output().expect("run integer executable");
    assert!(run.status.success());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_zero_float_policy_rejects_float_builds_with_function_context() {
    let root = std::env::temp_dir().join(format!("actus-no-float-float-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"float_app\"\nversion = \"0.1.0\"\n\n[build]\nverify_no_float_ir = true\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/main.act"),
        "verb main() -> Int { erg value: f32 = 1.0f32; return 0; }\n",
    )
    .expect("write float source");

    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "obj"])
        .current_dir(&root)
        .output()
        .expect("run float zero-float build");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("native function `main`"), "stderr: {stderr}");
    assert!(stderr.contains("floating-point IR instructions"), "stderr: {stderr}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_templates_reject_freestanding_without_a_target() {
    let root = std::env::temp_dir().join(format!("actus-new-bare-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["new", root.to_str().unwrap(), "--runtime", "freestanding", "--no-git"])
        .output()
        .expect("run new command");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires an explicit target"));
    assert!(!root.exists());
}

#[test]
fn init_prints_next_steps_and_initializes_main() {
    let root = std::env::temp_dir().join(format!("actus-init-main-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create init directory");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["init"])
        .current_dir(&root)
        .output()
        .expect("run init command");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("actus run"));
    assert_main_branch_or_parent_repository(&root);
    let _ = fs::remove_dir_all(root);
}

fn assert_main_branch_or_parent_repository(root: &Path) {
    let parent_repository =
        root.ancestors().skip(1).any(|directory| directory.join(".git").exists());
    if parent_repository {
        assert!(!root.join(".git").exists());
        return;
    }
    let branch = Command::new("git")
        .args(["-C", root.to_str().unwrap(), "symbolic-ref", "--short", "HEAD"])
        .output()
        .expect("read initial branch");
    assert!(branch.status.success());
    assert_eq!(String::from_utf8_lossy(&branch.stdout).trim(), "main");
}

#[test]
fn init_creates_an_existing_directory_project_without_git() {
    let root = std::env::temp_dir().join(format!("actus-init-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create init directory");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["init", "--no-git"])
        .current_dir(&root)
        .status()
        .expect("run init command");

    assert!(status.success());
    assert!(root.join("Actus.toml").is_file());
    assert!(root.join("src/main.act").is_file());
    assert!(!root.join(".git").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_validates_source_without_emitting_code() {
    let input = std::env::temp_dir().join(format!("actus-check-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

    let result = run_with_args(vec!["check".to_owned(), input.display().to_string()].into_iter());

    assert_eq!(result, 0);
    assert!(!input.with_extension("o").exists());
    let _ = fs::remove_file(input);
}

#[test]
fn check_validates_a_module_sibling_through_its_facade() {
    let root = std::env::temp_dir().join(format!("actus-check-module-{}", std::process::id()));
    let module = root.join("src/path");
    fs::create_dir_all(&module).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"module-check\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(module.join("path.act"), "open types;\nopen use;\n").expect("write module facade");
    fs::write(module.join("types.act"), "open struct Token { erg byte: Int, }\n")
        .expect("write type sibling");
    fs::write(
        module.join("use.act"),
        "open verb inspect(abs token: Token) -> Int { return token.byte; }\n",
    )
    .expect("write dependent sibling");

    let result = run_with_args(
        vec!["check".to_owned(), module.join("use.act").display().to_string()].into_iter(),
    );

    assert_eq!(result, 0);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn native_build_propagates_nested_generic_instances_through_an_imported_module() {
    let root =
        std::env::temp_dir().join(format!("actus-nested-generic-module-{}", std::process::id()));
    let module = root.join("src/aie");
    fs::create_dir_all(&module).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"nested-generic-module\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "import aie; verb main() -> Int { erg storage: Storage[4] = Storage[4] { values: Array[u32, 4](), }; return outer(storage: ins storage) as Int; }\n")
        .expect("write root source");
    fs::write(module.join("aie.act"), "open engine;\n").expect("write module facade");
    fs::write(
        module.join("engine.act"),
        "open struct Storage[N: Usize] { erg values: Array[u32, N], } open verb inner[N: Usize](ins storage: Storage[N]) -> u32 { return N as u32; } open verb outer[N: Usize](ins storage: Storage[N]) -> u32 { return inner(storage: ins storage); }\n",
    )
    .expect("write generic module source");
    let output = root.join("nested-generic-module");
    let result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o", output.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .expect("build imported nested generic module");
    assert!(result.status.success(), "stderr: {}", String::from_utf8_lossy(&result.stderr));
    let status = Command::new(&output).status().expect("run nested generic module");
    assert_eq!(status.code(), Some(4));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn watch_once_checks_the_project_entry() {
    let root = std::env::temp_dir().join(format!("actus-watch-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"watcher\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main() -> Int { return 0; }\n")
        .expect("write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["watch", "--once"])
        .current_dir(&root)
        .output()
        .expect("run watch command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("watching `") && stdout.contains("main.act`"));
    assert!(stdout.contains("checked `") && stdout.contains("main.act` successfully"));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn check_and_run_follow_manifest_source_root_and_entry() {
    let root = std::env::temp_dir().join(format!("actus-custom-entry-{}", std::process::id()));
    fs::create_dir_all(root.join("app")).expect("create source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"custom\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nsource_root = \"app\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("app/main.act"), "verb main() -> Int { return 44; }\n")
        .expect("write entry source");

    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("check")
        .current_dir(&root)
        .output()
        .expect("run check command");
    assert!(check.status.success(), "stderr: {}", String::from_utf8_lossy(&check.stderr));
    assert!(String::from_utf8_lossy(&check.stdout).contains("app/main.act"));

    let run = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("run")
        .current_dir(&root)
        .output()
        .expect("run project");
    assert_eq!(
        run.status.code(),
        Some(44),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn watch_rechecks_after_a_source_change_and_surfaces_failure() {
    let root = std::env::temp_dir().join(format!("actus-watch-change-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"watcher\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    let source = root.join("src/main.act");
    fs::write(&source, "verb main() -> Int { return 0; }\n").expect("write entry source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["watch", "--interval", "20"])
        .current_dir(&root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("start watch command");
    thread::sleep(Duration::from_millis(100));
    fs::write(&source, "verb main() -> Int { return ; }\n").expect("write invalid source");
    thread::sleep(Duration::from_millis(150));
    let _ = child.kill();
    let output = child.wait_with_output().expect("collect watch output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("change detected"), "stdout: {stdout}");
    assert!(stderr.contains("error[") || stderr.contains("expected"), "stderr: {stderr}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn build_discovers_the_manifest_entry_without_an_input_path() {
    let root = std::env::temp_dir().join(format!("actus-build-entry-{}", std::process::id()));
    let output = root.join("hello");
    fs::create_dir_all(root.join("src")).expect("create source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"entry\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main() -> Int { return 42; }\n")
        .expect("write entry source");

    let result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&root)
        .output()
        .expect("run build command");

    assert!(
        result.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn build_discovers_a_parent_manifest_from_a_nested_directory() {
    let root = std::env::temp_dir().join(format!("actus-build-nested-{}", std::process::id()));
    let nested = root.join("src").join("nested");
    let output = nested.join("hello");
    fs::create_dir_all(&nested).expect("create nested source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"nested\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "verb main() -> Int { return 43; }\n")
        .expect("write entry source");

    let result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&nested)
        .output()
        .expect("run nested build command");

    assert!(
        result.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(43));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn run_accepts_release_profile_and_forwards_arguments() {
    let input = std::env::temp_dir().join(format!("actus-run-options-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

    let result = run_with_args(
        vec![
            "run".to_owned(),
            input.display().to_string(),
            "--release".to_owned(),
            "--".to_owned(),
            "argument".to_owned(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 42);
    let _ = fs::remove_file(input);
}

#[cfg(unix)]
#[test]
fn run_preserves_program_output_and_reports_status_without_temp_path() {
    let input = std::env::temp_dir().join(format!("actus-run-output-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { print(42); return 7; }\n").expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["run", input.to_str().unwrap()])
        .output()
        .expect("run source");

    assert_eq!(output.status.code(), Some(7));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("process exited with status 7"));
    assert!(!stderr.contains("/tmp/actus-run-"));
    assert!(!output.stdout.windows(5).any(|window| window == b"built"));
    let _ = fs::remove_file(input);
}

#[cfg(unix)]
#[test]
fn run_reports_build_failures_without_exposing_temp_paths() {
    let input =
        std::env::temp_dir().join(format!("actus-invalid-source-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return ; }\n").expect("write invalid source");

    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["run", input.to_str().unwrap()])
        .output()
        .expect("run invalid source");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("process exited with status"));
    assert!(!stderr.contains("/tmp/actus-run-"));
    assert!(!output.stdout.windows(5).any(|window| window == b"built"));
    let _ = fs::remove_file(input);
}

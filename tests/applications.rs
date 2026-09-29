use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn copy_standard_library(root: &Path) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let files = [
        "lib.act",
        "io/io.act",
        "io/error.act",
        "io/reader.act",
        "io/writer.act",
        "io/stdout.act",
        "io/stderr.act",
        "io/stdin.act",
        "io/read.act",
        "io/write.act",
        "io/buffered.act",
        "io/cursor.act",
        "io/copy.act",
        "fs/fs.act",
        "fs/file.act",
        "fs/metadata.act",
        "fs/operations.act",
        "fs/options.act",
        "fs/seek.act",
        "path/path.act",
        "path/types.act",
        "path/storage.act",
        "path/error.act",
        "path/components.act",
        "path/posix.act",
        "path/windows.act",
        "path/predicates.act",
        "path/normalize.act",
        "path/builders.act",
    ];
    for relative in files {
        let destination = root.join("src").join(relative);
        fs::create_dir_all(destination.parent().expect("library parent"))
            .expect("library directory");
        fs::copy(source.join(relative), destination).expect("copy standard-library source");
    }
}

fn project(name: &str, source: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("actus-{name}-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create application project");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write application manifest");
    copy_standard_library(&root);
    let input = root.join("src/main.act");
    fs::write(&input, source).expect("write application source");
    (root.clone(), input, root.join("application"))
}

fn build(root: &Path, input: &Path, output: &Path) {
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(root)
        .output()
        .expect("check application");
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "exe", "-o"])
        .arg(output)
        .current_dir(root)
        .output()
        .expect("build application");
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
}

fn run(root: &Path, output: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(output)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run application");
    child
        .stdin
        .take()
        .expect("application stdin")
        .write_all(input)
        .expect("write application input");
    child.wait_with_output().expect("wait for application")
}

#[test]
fn console_application_proves_io_success_and_eof_contracts() {
    let source = include_str!("../examples/console_application.act");
    let (root, input, output) = project("console-application", source);
    build(&root, &input, &output);
    let success = run(&root, &output, b"hello\n");
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(success.stdout, b"hello\n");
    assert_eq!(success.stderr, b"hello\n");
    let eof = run(&root, &output, b"");
    assert_eq!(eof.status.code(), Some(2));
    assert_eq!(eof.stdout, b"");
    assert_eq!(eof.stderr, b"EO\n");
    fs::remove_dir_all(root).expect("remove console project");
}

#[test]
fn file_utility_proves_path_fs_success_and_typed_failure() {
    let source = include_str!("../examples/file_utility.act");
    let (root, input, output) = project("file-utility", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"ACT");
    assert_eq!(execution.stderr, b"");
    assert!(!root.join("actus_192.bin").exists());
    fs::remove_dir_all(root).expect("remove file utility project");
}

#[test]
fn negative_application_proves_missing_file_is_typed() {
    let source = include_str!("fixtures/applications/missing_file.act");
    let (root, input, output) = project("missing-file", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"");
    assert_eq!(execution.stderr, b"");
    fs::remove_dir_all(root).expect("remove negative application project");
}

#[test]
fn module_application_executes_public_wrapper_with_private_implementation() {
    let source = "import math; verb main() -> Int { return add(); }\n";
    let (root, input, output) = project("module-wrapper", source);
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write module facade");
    fs::write(
        root.join("src/math/api.act"),
        "open verb add() -> Int { return hidden(); } verb hidden() -> Int { return 42; }\n",
    )
    .expect("write module implementation");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    assert_eq!(execution.stdout, b"");
    assert_eq!(execution.stderr, b"");
    fs::remove_dir_all(root).expect("remove module project");
}

#[test]
fn object_build_emits_one_root_and_one_imported_module_object() {
    let source = "import math; verb main() -> Int { return add(); }\n";
    let (root, input, output) = project("module-objects", source);
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write module facade");
    fs::write(
        root.join("src/math/api.act"),
        "open verb add() -> Int { return hidden(); } verb hidden() -> Int { return 42; }\n",
    )
    .expect("write module implementation");
    let object = output.with_extension("obj");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build module objects");
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
    assert!(object.exists());
    assert!(root.join("application.module.actus_mod_4_math.obj").exists());
    fs::remove_dir_all(root).expect("remove module object project");
}

#[test]
fn module_application_rejects_private_bridge_before_codegen() {
    let source = "import runtime; verb main() -> Int { return private_bridge(); }\n";
    let (root, input, _output) = project("private-bridge", source);
    fs::create_dir_all(root.join("src/runtime")).expect("create runtime module directory");
    fs::write(root.join("src/runtime/runtime.act"), "open api;\n").expect("write runtime facade");
    fs::write(
        root.join("src/runtime/api.act"),
        "open verb read() -> Int { return private_bridge(); } unsafe extern \"C\" verb private_bridge() -> Int;\n",
    )
    .expect("write runtime implementation");
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(&root)
        .output()
        .expect("check private bridge application");
    assert!(!check.status.success());
    let diagnostics = String::from_utf8_lossy(&check.stderr);
    assert!(diagnostics.contains("E1109"), "diagnostics: {diagnostics}");
    fs::remove_dir_all(root).expect("remove private bridge project");
}

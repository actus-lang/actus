use super::support::{build, project, run};
use std::fs;
#[test]
fn console_application_proves_io_success_and_eof_contracts() {
    let source = include_str!("../../examples/console_application.act");
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
    let source = include_str!("../../examples/file_utility.act");
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
    let source = include_str!("../fixtures/applications/missing_file.act");
    let (root, input, output) = project("missing-file", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"");
    assert_eq!(execution.stderr, b"");
    fs::remove_dir_all(root).expect("remove negative application project");
}

#[test]
fn temporary_file_drop_removes_resource_after_early_exit() {
    let source = r#"
import {
    open fs,
    open path,
};

verb main() -> Int {
    erg raw: Buffer = Buffer[0];
    append(raw, 46u8);
    append(raw, 0u8);
    erg root_result = path_from_ascii(storage: dat raw);
    return case dat root_result {
        Result.Err(_) => 1,
        Result.Ok(root) => {
            erg prefix: Buffer = Buffer[0];
            append(prefix, 97u8);
            append(prefix, 99u8);
            append(prefix, 116u8);
            append(prefix, 117u8);
            append(prefix, 115u8);
            append(prefix, 45u8);
            append(prefix, 100u8);
            append(prefix, 114u8);
            append(prefix, 111u8);
            append(prefix, 112u8);
            erg created = temporary_file_open(root: abs root, prefix: abs prefix);
            return case dat created {
                Result.Err(_) => 2,
                Result.Ok(_resource) => 0,
            };
        },
    };
}
"#;
    let (root, input, output) = project("temporary-drop", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    let leaked = fs::read_dir(&root)
        .expect("read temporary-drop project")
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("actus-drop-"));
    assert!(!leaked, "temporary file survived Drop cleanup");
    fs::remove_dir_all(root).expect("remove temporary-drop project");
}

#[test]
fn module_application_executes_public_wrapper_with_private_implementation() {
    let source = "import { open math }; verb main() -> Int { return add(); }\n";
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

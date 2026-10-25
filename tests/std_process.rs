use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use actus::cli::run_with_args;

fn library_source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/process").join(file);
    fs::read_to_string(path).expect("standard process source should exist")
}

fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root =
        std::env::temp_dir().join(format!("actus-std-process-{name}-{}", std::process::id()));
    let process_root = root.join("src/process");
    fs::create_dir_all(&process_root).expect("create process fixture");
    for file in ["process.act", "arguments.act", "error.act"] {
        fs::write(process_root.join(file), library_source(file)).expect("copy process source");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-process-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write fixture manifest");
    let input = root.join("src/main.act");
    fs::write(&input, fixture_source()).expect("write process fixture entry");
    (root.clone(), input, root.join(name))
}

fn fixture_source() -> &'static str {
    r#"import { open process };
verb main() -> Int {
    erg created = arguments();
    return case dat created {
        Result.Err(_) => 1,
        Result.Ok(state) => {
            if count(arguments: abs state) != 3 {
                return 2;
            }
            erg buffer = Buffer[4096];
            erg first = next_arg(arguments: ins state, buffer: ins buffer);
            erg second = next_arg(arguments: ins state, buffer: ins buffer);
            erg third = next_arg(arguments: ins state, buffer: ins buffer);
            drop(first);
            drop(third);
            return case dat second {
                Result.Err(_) => 3,
                Result.Ok(value) => {
                    erg length: Option[Int] = value;
                    return case dat length {
                        Option.Some(byte_count) => if byte_count == 5 {
                            0
                        } else {
                            5;
                        },
                        Option.None => 6,
                    };
                },
            };
        },
    };
}
"#
}

fn build(input: &Path, output: &Path, backend: &str) {
    assert_eq!(
        run_with_args(
            vec![
                "build".to_owned(),
                input.display().to_string(),
                "--emit".to_owned(),
                "exe".to_owned(),
                "--backend".to_owned(),
                backend.to_owned(),
                "-o".to_owned(),
                output.display().to_string(),
            ]
            .into_iter(),
        ),
        0
    );
}

fn assert_process_arguments(backend: &str) {
    let (root, input, output) = project(backend);
    build(&input, &output, backend);
    let execution = Command::new(&output)
        .arg("alpha")
        .arg("omega")
        .output()
        .expect("process fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_process_native_arguments_work_on_cranelift() {
    assert_process_arguments("cranelift");
}

#[test]
#[cfg(feature = "llvm")]
fn std_process_native_arguments_work_on_llvm() {
    assert_process_arguments("llvm");
}

#[test]
fn std_process_is_rejected_on_freestanding_targets() {
    let root =
        std::env::temp_dir().join(format!("actus-std-process-freestanding-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create freestanding process fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-process-freestanding\"\nversion = \"0.1.0\"\nsource_root = \"src\"\nentry = \"boot\"\n\n[build]\nruntime = \"std\"\ntarget = \"x86_64-unknown-none\"\nentry_contract = \"freestanding\"\n",
    )
    .expect("write freestanding process manifest");
    let input = source_root.join("main.act");
    fs::write(&input, "import std::process; verb boot() -> Int { return 0; }\n")
        .expect("write freestanding process source");
    assert_eq!(
        run_with_args(
            vec!["check".to_owned(), input.display().to_string(), "--strict".to_owned()]
                .into_iter(),
        ),
        1
    );
    let _ = fs::remove_dir_all(root);
}

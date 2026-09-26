use std::fs;
use std::path::PathBuf;
use std::process::Command;

use actus::cli::run_with_args;
use actus::modules::{ModuleResolver, analyze_module, exports_module};

fn library_source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/io").join(file);
    fs::read_to_string(path).expect("standard library source should exist")
}

#[test]
fn std_io_declarations_pass_semantic_validation() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let resolver = ModuleResolver::new(source_root);
    let exports = exports_module(&resolver, "io").expect("std io facade should resolve");
    assert!(exports.contains("verb", "print_int"));
    assert!(exports.contains("verb", "eprint_int"));
    analyze_module(&resolver, "io").expect("std io declarations should be semantically valid");
}

#[test]
fn std_io_native_bridge_separates_stdout_and_stderr() {
    let root = std::env::temp_dir().join(format!("actus-std-io-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create std io fixture");
    for file in ["io.act", "stdout.act", "stderr.act"] {
        fs::write(
            io_root.join(file),
            if file == "io.act" {
                "open stdout;\nopen stderr;\n".to_owned()
            } else {
                library_source(file)
            },
        )
        .expect("copy std io source");
    }
    let input = source_root.join("main.act");
    let output = root.join("std-io");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-io-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write fixture manifest");
    fs::write(
        &input,
        "import io; verb main() -> Int { erg value = 42; print_int(value: value); eprint_int(value: value); return 0; }\n",
    )
    .expect("write std io entry");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);

    let execution = Command::new(&output).output().expect("std io fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"42\n");
    assert_eq!(execution.stderr, b"42\n");
    let _ = fs::remove_dir_all(root);
}

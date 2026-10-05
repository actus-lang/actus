use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_actus"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/phase-29/external-text-consumer")
}

fn writable_fixture_root() -> PathBuf {
    let source = fixture_root();
    let root =
        std::env::temp_dir().join(format!("actus-external-text-consumer-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create writable fixture source directory");
    fs::create_dir_all(root.join("tests")).expect("create writable fixture test directory");
    for relative_path in ["Actus.toml", "Actus.lock", "src/main.act", "tests/text_boundary.act"] {
        fs::copy(source.join(relative_path), root.join(relative_path))
            .unwrap_or_else(|error| panic!("copy fixture `{relative_path}`: {error}"));
    }
    root
}

#[test]
fn generic_external_text_consumer_passes_all_native_boundaries() {
    let root = writable_fixture_root();
    let output = root.join("target/external-text-consumer");
    let object = root.join("target/external-text-consumer.o");
    let repeated_object = root.join("target/external-text-consumer-repeat.o");

    let check = Command::new(compiler())
        .args(["check", "--strict"])
        .current_dir(&root)
        .output()
        .expect("strict check");
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));

    let tests = Command::new(compiler())
        .args(["test", "--strict"])
        .current_dir(&root)
        .output()
        .expect("strict test runner");
    assert!(tests.status.success(), "{}", String::from_utf8_lossy(&tests.stderr));
    assert!(String::from_utf8_lossy(&tests.stdout).contains("1 passed"));

    let object_build = Command::new(compiler())
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("object build");
    assert!(object_build.status.success(), "{}", String::from_utf8_lossy(&object_build.stderr));
    assert!(
        String::from_utf8_lossy(&object_build.stdout).contains("no floating-point instructions")
    );

    let executable_build = Command::new(compiler())
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&root)
        .output()
        .expect("executable build");
    assert!(
        executable_build.status.success(),
        "{}",
        String::from_utf8_lossy(&executable_build.stderr)
    );

    let run = Command::new(&output).output().expect("run executable");
    assert_eq!(run.status.code(), Some(0));
    assert!(object.exists());
    assert!(output.exists());

    let repeated_build = Command::new(compiler())
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&repeated_object)
        .current_dir(&root)
        .output()
        .expect("repeated object build");
    assert!(repeated_build.status.success(), "{}", String::from_utf8_lossy(&repeated_build.stderr));
    assert_eq!(
        fs::read(&object).expect("read object"),
        fs::read(&repeated_object).expect("read repeated object")
    );

    let _ = fs::remove_dir_all(root);
}

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_actus"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/phase-29/external-text-consumer")
}

#[test]
fn generic_external_text_consumer_passes_all_native_boundaries() {
    let root = fixture_root();
    let output = root.join("target/external-text-consumer");
    let object = root.join("target/external-text-consumer.o");

    let check = Command::new(compiler())
        .args(["check", "--strict"])
        .current_dir(&root)
        .output()
        .expect("strict check");
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));

    let object_build = Command::new(compiler())
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("object build");
    assert!(object_build.status.success(), "{}", String::from_utf8_lossy(&object_build.stderr));

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

    let _ = fs::remove_dir_all(root.join("target"));
}

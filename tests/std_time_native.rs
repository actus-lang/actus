use std::fs;
use std::process::Command;

#[test]
fn hosted_std_time_bridge_builds_and_runs() {
    let root = std::env::temp_dir().join(format!("actus-std-time-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("time-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb main() -> Int { erg started: u64 = monotonic_nanos(); erg finished: u64 = monotonic_nanos(); if finished >= started { return 0; } return 1; }",
    )
    .expect("fixture source should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "hosted std time build failed: {status}");

    let execution = Command::new(&output).output().expect("time executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

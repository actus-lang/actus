use std::fs;
use std::process::Command;

use object::{Object, ObjectSymbol};

#[test]
fn hosted_std_time_bridge_builds_and_runs() {
    let root = std::env::temp_dir().join(format!("actus-std-time-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("time-example");
    let object_output = root.join("time-example.o");
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

    let object_status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "obj",
            "-o",
            object_output.to_str().expect("object path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus object build should start");
    assert!(object_status.success(), "hosted std time object build failed: {object_status}");
    let object_bytes = fs::read(&object_output).expect("time object should be readable");
    let object_file =
        object::File::parse(object_bytes.as_slice()).expect("time object should parse");
    let bridge_symbols = object_file
        .symbols()
        .filter_map(|symbol| symbol.name().ok())
        .filter(|name| {
            name.trim_start_matches('_') == "actus_mod_3_std_4_time__verb_monotonic_5fnanos"
        })
        .count();
    assert_eq!(bridge_symbols, 1, "timer facade dependency must be registered exactly once");

    let execution = Command::new(&output).output().expect("time executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn freestanding_std_time_is_rejected_before_native_emission() {
    let root =
        std::env::temp_dir().join(format!("actus-std-time-freestanding-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_freestanding\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\ntarget = \"x86_64-unknown-none\"\n",
    )
    .expect("fixture manifest should be written");
    fs::write(source_root.join("main.act"), "import std::time; verb main() -> Int { return 0; }")
        .expect("fixture source should be written");

    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "check",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
        ])
        .output()
        .expect("Actus check should start");
    assert!(!check.status.success());
    assert!(String::from_utf8_lossy(&check.stderr).contains("E1112"));
    let _ = fs::remove_dir_all(root);
}

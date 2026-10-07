use std::fs;
use std::process::Command;

#[test]
fn hosted_std_region_operations_build_and_run() {
    let root = std::env::temp_dir().join(format!("actus-std-region-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::region; verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg destination: Buffer = Buffer[4]; erg index: u64 = 0u64; erg read = region_read(region: abs region, index: erg index, destination: ins destination); return case dat read { Result.Err(_) => 2, Result.Ok(_) => 0, }; }, }; }",
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
    assert!(status.success(), "hosted std region build failed: {status}");

    let execution = Command::new(&output).output().expect("region executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_std_region_write_publish_and_read_are_visible() {
    let root = std::env::temp_dir().join(format!("actus-std-region-write-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-write-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_write_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::region; verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg source: Buffer = Buffer[4]; erg index: u64 = 0u64; erg written = region_write(region: ins region, index: erg index, source: abs source); return case dat written { Result.Err(_) => 2, Result.Ok(_) => { erg published = region_publish(region: ins region); return case dat published { Result.Err(_) => 3, Result.Ok(_) => { erg destination: Buffer = Buffer[4]; erg read = region_read(region: abs region, index: erg index, destination: ins destination); return case dat read { Result.Err(_) => 4, Result.Ok(_) => { erg closed = region_close(region: ins region); drop(closed); return 0; }, }; }, }; }, }; }, }; }",
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
    assert!(status.success(), "hosted std region write build failed: {status}");

    let execution = Command::new(&output).output().expect("region executable should run");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

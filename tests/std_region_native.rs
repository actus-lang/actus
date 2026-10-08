use std::fs;
use std::process::Command;

use object::{Object, ObjectSection, ObjectSymbol};

fn write_region_object_fixture(root: &std::path::Path, logical_length: &str) {
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_object_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        format!(
            "import std::region; verb main() -> Int {{ erg backing: Buffer = Buffer[4]; erg logical_length: u64 = {logical_length}; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); drop(opened); return 0; }}\n"
        ),
    )
    .expect("fixture source should be written");
}

fn write_inline_object_fixture(root: &std::path::Path) {
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("inline fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"inline_object_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("inline fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "verb main() -> Int { erg values: Array[u32, 2] = Array[u32, 2](); return values[0u32] as Int; }\n",
    )
    .expect("inline fixture source should be written");
}

fn build_region_object(root: &std::path::Path) -> Vec<u8> {
    let input = root.join("src/main.act");
    let output = root.join("region.o");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            input.to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "obj",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .output()
        .expect("Actus object build should start");
    assert!(
        build.status.success(),
        "hosted Region object build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    fs::read(output).expect("read generated Region object")
}

fn build_object(root: &std::path::Path) -> Vec<u8> {
    let input = root.join("src/main.act");
    let output = root.join("main.o");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            input.to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "obj",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .output()
        .expect("Actus object build should start");
    assert!(
        build.status.success(),
        "inline object build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    fs::read(output).expect("read generated inline object")
}

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
fn hosted_std_region_inspection_and_clean_remap_run_natively() {
    let root = std::env::temp_dir().join(format!("actus-std-region-remap-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-remap-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_remap_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        r#"import std::region;
verb check_start(abs region: Region[u32]) -> Int {
    erg result = region_window_start(region: abs region);
    return case dat result { Result.Err(_) => 1, Result.Ok(value) => if value == 0u64 { return 0; } else { return 2; }, };
}

verb check_generation(abs region: Region[u32]) -> Int {
    erg result = region_generation(region: abs region);
    return case dat result { Result.Err(_) => 1, Result.Ok(value) => if value == 2u64 { return 0; } else { return 2; }, };
}
verb main() -> Int {
    erg backing: Buffer = Buffer[4];
    erg logical_length: u64 = 4u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 20,
        Result.Ok(region) => {
            erg before = check_start(region: abs region);
            if before != 0 { return 21; }
            erg replacement: Buffer = Buffer[8];
            erg replacement_start: u64 = 2u64;
            erg replacement_count: u64 = 2u64;
            erg remapped = region_remap(region: ins region, backing: dat replacement, window_start: erg replacement_start, window_count: erg replacement_count);
            return case dat remapped {
                Result.Err(_) => 22,
                Result.Ok(generation) => {
                    if generation != 2u64 { return 23; }
                    erg after = check_generation(region: abs region);
                    if after != 0 { return 24; }
                    erg closed = region_close(region: ins region);
                    return case dat closed { Result.Err(_) => 25, Result.Ok(_) => 0, };
                },
            };
        },
    };
}
"#,
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
    assert!(status.success(), "hosted Region remap build failed: {status}");

    let execution = Command::new(&output).output().expect("Region remap executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_std_region_bounded_ranges_run_natively() {
    let root = std::env::temp_dir().join(format!("actus-std-region-range-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-range-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_range_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        r#"import std::region;
verb verify_published(ins region: Region[u32]) -> Int {
    erg published = region_publish(region: ins region);
    return case dat published {
        Result.Err(_) => 6,
        Result.Ok(_) => {
            erg destination: Buffer = Buffer[8];
            erg read_start: u64 = 0u64;
            erg range_count: u64 = 2u64;
            erg read = region_read_range(region: abs region, start_index: erg read_start, element_count: erg range_count, destination: ins destination);
            return case dat read {
                Result.Err(_) => 7,
                Result.Ok(count) => {
                    if count != 2u64 { return 8; }
                    erg empty: Buffer = Buffer[0];
                    erg empty_start: u64 = 4u64;
                    erg empty_count: u64 = 0u64;
                    erg no_op = region_read_range(region: abs region, start_index: erg empty_start, element_count: erg empty_count, destination: ins empty);
                    return case dat no_op { Result.Err(_) => 9, Result.Ok(empty_result) => if empty_result == 0u64 { return 0; } else { return 10; }, };
                },
            };
        },
    };
}
verb verify_ranges(ins region: Region[u32], abs source: Buffer) -> Int {
    erg crossing_start: u64 = 1u64;
    erg range_count: u64 = 2u64;
    erg crossing = region_write_range(region: ins region, start_index: erg crossing_start, element_count: erg range_count, source: abs source);
    erg crossed = case dat crossing { Result.Err(_) => 0, Result.Ok(_) => 2, };
    if crossed != 0 { return 3; }
    erg write_start: u64 = 0u64;
    erg written = region_write_range(region: ins region, start_index: erg write_start, element_count: erg range_count, source: abs source);
    return case dat written { Result.Err(_) => 4, Result.Ok(count) => if count == 2u64 { return verify_published(region: ins region); } else { return 5; }, };
}
verb main() -> Int {
    erg backing: Buffer = Buffer[8];
    erg source: Buffer = Buffer[8];
    erg logical_length: u64 = 4u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 2u64;
    erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { return verify_ranges(region: ins region, source: abs source); }, };
}
"#,
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
    assert!(status.success(), "hosted Region range build failed: {status}");

    let execution = Command::new(&output).output().expect("Region range executable should run");
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

#[test]
fn hosted_std_region_rejects_out_of_window_read_without_trap() {
    let root = std::env::temp_dir().join(format!("actus-std-region-bounds-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-bounds-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_bounds_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::region; verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg destination: Buffer = Buffer[4]; erg index: u64 = 1u64; erg read = region_read(region: abs region, index: erg index, destination: ins destination); return case dat read { Result.Err(_) => 0, Result.Ok(_) => 2, }; }, }; }\n",
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
        .expect("Actus bounds build should start");
    assert!(status.success(), "hosted Region bounds build failed: {status}");

    let execution = Command::new(&output).output().expect("bounds executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_std_region_pinning_blocks_remap_until_unpinned() {
    let root =
        std::env::temp_dir().join(format!("actus-std-region-pinning-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-pinning-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_pinning_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::region; verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg pinned = region_pin(region: ins region); return case dat pinned { Result.Err(_) => 2, Result.Ok(_) => { erg state = region_pinned(region: abs region); return case dat state { Result.Err(_) => 3, Result.Ok(value) => { if value != 1 { return 4; } erg replacement: Buffer = Buffer[4]; erg remapped = region_remap(region: ins region, backing: dat replacement, window_start: erg window_start, window_count: erg window_count); return case dat remapped { Result.Ok(_) => 5, Result.Err(_) => { erg unpinned = region_unpin(region: ins region); return case dat unpinned { Result.Err(_) => 6, Result.Ok(_) => { erg closed = region_close(region: ins region); return case dat closed { Result.Err(_) => 7, Result.Ok(_) => 0, }; }, }; }, }; }, }; }, }; }, }; }",
    )
    .expect("fixture source should be written");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .output()
        .expect("Actus pinning fixture should build");
    assert!(
        build.status.success(),
        "pinning fixture build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&output).output().expect("pinning fixture should run");
    assert!(run.status.success(), "pinning fixture returned {:?}", run.status);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_std_region_rejects_access_after_close_without_trap() {
    let root = std::env::temp_dir().join(format!("actus-std-region-closed-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-closed-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_closed_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::region; verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg closed = region_close(region: ins region); return case dat closed { Result.Err(_) => 2, Result.Ok(_) => { erg destination: Buffer = Buffer[4]; erg index: u64 = 0u64; erg read = region_read(region: abs region, index: erg index, destination: ins destination); return case dat read { Result.Err(_) => 0, Result.Ok(_) => 3, }; }, }; }, }; }\n",
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
        .expect("Actus closed-region build should start");
    assert!(status.success(), "hosted closed Region build failed: {status}");

    let execution = Command::new(&output).output().expect("closed-region executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_std_region_cleanup_handles_early_return_and_nested_transfer() {
    let root =
        std::env::temp_dir().join(format!("actus-std-region-cleanup-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-cleanup-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_cleanup_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        r#"import std::region;
verb consume_region(dat region: Region[u32]) -> Int {
    return 0;
}
verb open_and_return(dat backing: Buffer) -> Int {
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { return 0; }, };
}
verb open_and_transfer(dat backing: Buffer) -> Int {
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened { Result.Err(_) => 2, Result.Ok(region) => { return consume_region(region: dat region); }, };
}
verb main() -> Int {
    erg first: Buffer = Buffer[4];
    erg early = open_and_return(backing: dat first);
    if early != 0 { return 3; }
    erg second: Buffer = Buffer[4];
    return open_and_transfer(backing: dat second);
}
"#,
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
    assert!(status.success(), "hosted Region cleanup build failed: {status}");

    let execution = Command::new(&output).output().expect("Region cleanup executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_region_object_has_bounded_abi_sections_and_relocations() {
    let small_root =
        std::env::temp_dir().join(format!("actus-region-object-small-{}", std::process::id()));
    let large_root =
        std::env::temp_dir().join(format!("actus-region-object-large-{}", std::process::id()));
    write_region_object_fixture(&small_root, "1u64");
    write_region_object_fixture(&large_root, "4294967296u64");

    let small_bytes = build_region_object(&small_root);
    let large_bytes = build_region_object(&large_root);
    let small = object::File::parse(small_bytes.as_slice()).expect("parse small Region object");
    let large = object::File::parse(large_bytes.as_slice()).expect("parse large Region object");

    for file in [&small, &large] {
        assert!(file.sections().any(|section| section.kind() == object::SectionKind::Text));
        let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
        assert!(symbols.iter().all(|symbol| symbol.len() <= 128));
        assert!(symbols.iter().any(|symbol| symbol.contains("actus_region_drop")));
        assert!(symbols.iter().any(|symbol| symbol.contains("region_5fopen")));
        assert!(!symbols.iter().any(|symbol| symbol.contains("4294967296")));

        let relocation_count =
            file.sections().map(|section| section.relocations().count()).sum::<usize>();
        assert!(relocation_count <= 32, "unexpected relocation growth: {relocation_count}");

        let initialized_bytes = file
            .sections()
            .filter(|section| section.kind() == object::SectionKind::Data)
            .map(|section| section.size())
            .sum::<u64>();
        assert!(
            initialized_bytes <= 4096,
            "unexpected inline initializer growth: {initialized_bytes}"
        );
    }

    assert!(large_bytes.len() <= small_bytes.len() * 2, "logical extent inflated object size");

    let inline_root =
        std::env::temp_dir().join(format!("actus-inline-object-{}", std::process::id()));
    write_inline_object_fixture(&inline_root);
    let inline_bytes = build_object(&inline_root);
    let inline = object::File::parse(inline_bytes.as_slice()).expect("parse inline object");
    let inline_symbols =
        inline.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(!inline_symbols.iter().any(|symbol| symbol.contains("actus_region_open")));
    assert!(!inline_symbols.iter().any(|symbol| symbol.contains("region_5fopen")));
    assert!(
        small_bytes.len() > inline_bytes.len(),
        "runtime-backed Region must retain a distinct descriptor/bridge ABI"
    );

    let _ = fs::remove_dir_all(small_root);
    let _ = fs::remove_dir_all(large_root);
    let _ = fs::remove_dir_all(inline_root);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn freestanding_region_object_imports_only_target_provider_bridges() {
    let root =
        std::env::temp_dir().join(format!("actus-freestanding-region-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root)
        .expect("freestanding fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"freestanding_region_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nsource_root = \"src\"\nentry = \"boot\"\n\n[build]\nruntime = \"std\"\ntarget = \"x86_64-unknown-uefi\"\nentry_contract = \"freestanding\"\nverify_no_float_ir = true\n",
    )
    .expect("freestanding fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        r#"import std::region;
verb after_write(ins region: Region[u8], ins destination: Buffer) -> Int {
    erg published = region_publish(region: ins region);
    return case dat published {
        Result.Err(_) => 3,
        Result.Ok(_) => {
            erg cancelled = region_cancel(region: ins region);
            return case dat cancelled {
                Result.Err(_) => 4,
                Result.Ok(_) => {
                    erg index: u64 = 0u64;
                    erg read = region_read(region: abs region, index: erg index, destination: ins destination);
                    return case dat read {
                        Result.Err(_) => 5,
                        Result.Ok(_) => {
                            erg closed = region_close(region: ins region);
                            drop(closed);
                            return 0;
                        },
                    };
                },
            };
        },
    };
}
verb write_and_continue(ins region: Region[u8], abs source: Buffer, ins destination: Buffer) -> Int {
    erg index: u64 = 0u64;
    erg written = region_write(region: ins region, index: erg index, source: abs source);
    return case dat written {
        Result.Err(_) => 2,
        Result.Ok(_) => {
            return after_write(region: ins region, destination: ins destination);
        },
    };
}
verb boot(erg backing: Buffer, erg destination: Buffer, erg source: Buffer) -> Int {
    erg logical_length: u64 = 1099511627776u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[u8](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => {
            return write_and_continue(region: ins region, source: abs source, destination: ins destination);
        },
    };
}
"#,
    )
    .expect("freestanding fixture source should be written");

    let output = root.join("main.o");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "obj",
            "-o",
            output.to_str().expect("object path should be valid UTF-8"),
        ])
        .output()
        .expect("freestanding Region object build should start");
    assert!(
        build.status.success(),
        "freestanding Region object build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let mut defined = Vec::new();
    let mut undefined = Vec::new();
    for entry in fs::read_dir(&root).expect("read freestanding fixture directory") {
        let path = entry.expect("read fixture entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("o") {
            continue;
        }
        let bytes = fs::read(&path).expect("read generated freestanding object");
        let file =
            object::File::parse(bytes.as_slice()).expect("parse generated freestanding object");
        for symbol in file.symbols() {
            let Some(name) = symbol.name().ok().map(str::to_owned) else {
                continue;
            };
            if symbol.is_undefined() {
                undefined.push(name);
            } else {
                defined.push(name);
            }
        }
    }
    undefined.retain(|symbol| !defined.iter().any(|defined_symbol| defined_symbol == symbol));
    undefined.sort();
    undefined.dedup();
    assert_eq!(
        undefined,
        [
            "actus_buffer_drop",
            "actus_enum_drop",
            "actus_region_cancel",
            "actus_region_close",
            "actus_region_drop",
            "actus_region_open",
            "actus_region_publish",
            "actus_region_read",
            "actus_region_write",
        ]
    );

    let _ = fs::remove_dir_all(root);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn freestanding_region_object_scales_logical_capacity_with_bounded_output() {
    let logical_lengths = ["1048576u64", "4294967296u64", "1099511627776u64"];
    let mut object_sizes = Vec::new();

    for (case_index, logical_length) in logical_lengths.iter().enumerate() {
        let root = std::env::temp_dir()
            .join(format!("actus-freestanding-region-scale-{}-{case_index}", std::process::id()));
        let source_root = root.join("src");
        fs::create_dir_all(&source_root)
            .expect("freestanding scale fixture source directory should be created");
        fs::write(
            root.join("Actus.toml"),
            "[package]\nname = \"freestanding_region_scale_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nsource_root = \"src\"\nentry = \"boot\"\n\n[build]\nruntime = \"std\"\ntarget = \"x86_64-unknown-uefi\"\nentry_contract = \"freestanding\"\nverify_no_float_ir = true\n",
        )
        .expect("freestanding scale fixture manifest should be written");
        fs::write(
            source_root.join("main.act"),
            format!(
                "import std::region;\nverb boot(erg backing: Buffer) -> Int {{\n    erg logical_length: u64 = {logical_length};\n    erg window_start: u64 = 0u64;\n    erg window_count: u64 = 1u64;\n    erg opened = region_open[u8](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);\n    return case dat opened {{\n        Result.Err(_) => 1,\n        Result.Ok(region) => {{ drop(region); return 0; }},\n    }};\n}}\n"
            ),
        )
        .expect("freestanding scale fixture source should be written");

        let output = root.join("main.o");
        let build = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args([
                "build",
                source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
                "--strict",
                "--emit",
                "obj",
                "-o",
                output.to_str().expect("object path should be valid UTF-8"),
            ])
            .output()
            .expect("freestanding scale object build should start");
        assert!(
            build.status.success(),
            "freestanding scale object build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        object_sizes.push(fs::metadata(&output).expect("read scale object metadata").len());

        let mut defined = Vec::new();
        let mut undefined = Vec::new();
        for entry in fs::read_dir(&root).expect("read freestanding scale fixture directory") {
            let path = entry.expect("read scale fixture entry").path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("o") {
                continue;
            }
            let bytes = fs::read(&path).expect("read generated scale object");
            let file = object::File::parse(bytes.as_slice()).expect("parse generated scale object");
            for symbol in file.symbols() {
                let Some(name) = symbol.name().ok().map(str::to_owned) else {
                    continue;
                };
                if symbol.is_undefined() {
                    undefined.push(name);
                } else {
                    defined.push(name);
                }
            }
        }
        undefined.retain(|symbol| !defined.iter().any(|defined_symbol| defined_symbol == symbol));
        undefined.sort();
        undefined.dedup();
        assert_eq!(
            undefined,
            ["actus_buffer_drop", "actus_enum_drop", "actus_region_drop", "actus_region_open"]
        );
        let _ = fs::remove_dir_all(root);
    }

    let smallest = *object_sizes.iter().min().expect("scale objects should exist");
    let largest = *object_sizes.iter().max().expect("scale objects should exist");
    assert!(largest <= smallest * 2, "logical extent inflated object output: {object_sizes:?}");
}

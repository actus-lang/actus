use object::{Object, ObjectSymbol};
use std::fs;
use std::process::Command;

fn write_fixture(root: &std::path::Path, source: &str) {
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"region_aggregate_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(source_root.join("main.act"), source).expect("fixture source should be written");
}

fn build_and_run(root: &std::path::Path) -> std::process::Output {
    let input = root.join("src/main.act");
    let output = root.join("region-fixture");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            input.to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .output()
        .expect("Actus build should start");
    assert!(
        build.status.success(),
        "Region fixture build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    Command::new(output).output().expect("Region fixture should run")
}

fn build_object(root: &std::path::Path) -> Vec<u8> {
    let input = root.join("src/main.act");
    let output = root.join("region-fixture.o");
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
        "Region object build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    fs::read(output).expect("Region object should be readable")
}

#[test]
fn aggregate_region_fixture_passes_semantic_analysis_separately() {
    let source = "verb inspect(abs values: Region[Array[u8, 16]]) { }";
    let (tokens, errors) = actus::lexer::scan(source);
    assert!(errors.is_empty(), "semantic fixture lexing failed: {errors:?}");
    let program = actus::parser::parse(tokens).expect("semantic fixture should parse");
    actus::semantic::analyze(&program).expect("semantic fixture should analyze");
}

#[test]
fn aggregate_region_lifecycle_is_native() {
    let root = std::env::temp_dir().join(format!("actus-region-aggregate-{}", std::process::id()));
    write_fixture(
        &root,
        r#"import std::region;

verb finish_lifecycle(ins region: Region[Array[u8, 16]], abs source: Buffer) -> Int {
    erg index: u64 = 0u64;
    erg written = region_write(region: ins region, index: erg index, source: abs source);
    return case dat written {
        Result.Err(_) => 2,
        Result.Ok(_) => {
            erg published = region_publish(region: ins region);
            return case dat published {
                Result.Err(_) => 3,
                Result.Ok(_) => {
                    erg destination: Buffer = Buffer[16];
                    erg read = region_read(region: abs region, index: erg index, destination: ins destination);
                    return case dat read {
                        Result.Err(_) => 4,
                        Result.Ok(_) => {
                            erg closed = region_close(region: ins region);
                            return case dat closed {
                                Result.Err(_) => 5,
                                Result.Ok(_) => 0,
                            };
                        },
                    };
                },
            };
        },
    };
}

verb verify_layout() -> Int {
    erg aggregate_size: u64 = size_of[Array[u8, 16]]();
    erg aggregate_alignment: u64 = align_of[Array[u8, 16]]();
    if aggregate_size != 16u64 { return 6; }
    if aggregate_alignment != 1u64 { return 7; }
    return 0;
}

verb main() -> Int {
    erg layout_status: Int = verify_layout();
    if layout_status != 0 { return layout_status; }
    erg backing: Buffer = Buffer[16];
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[Array[u8, 16]](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => { erg source: Buffer = Buffer[16]; return finish_lifecycle(region: ins region, source: abs source); },
    };
}

"#,
    );
    let execution = build_and_run(&root);
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn aggregate_region_object_emission_exposes_runtime_bridges() {
    let root = std::env::temp_dir().join(format!("actus-region-object-{}", std::process::id()));
    write_fixture(
        &root,
        r#"import std::region;

verb main() -> Int {
    erg backing: Buffer = Buffer[16];
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[Array[u8, 16]](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    drop(opened);
    return 0;
}
"#,
    );
    let object = build_object(&root);
    let file = object::File::parse(object.as_slice()).expect("Region object should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(symbols.iter().any(|symbol| symbol.contains("region_5fopen")));
    assert!(symbols.iter().any(|symbol| symbol.contains("actus_region_drop")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn region_layout_registry_stays_consistent_across_semantic_intrinsics_and_runtime_stride() {
    let root = std::env::temp_dir()
        .join(format!("actus-region-layout-consistency-{}", std::process::id()));
    let source = r#"
import std::region;

struct Pair {
    first: u16,
    second: u32,
}

pack Frame {
    erg storage: Array[u8, 16];
    layout little;
    fields {
        erg marker: u8 at 0;
        abs _reserved: u120 at 8 = 0;
    }
}

verb open_and_close(dat backing: Buffer) -> Int {
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[Array[u8, 16]](
        backing: dat backing,
        logical_length: erg logical_length,
        window_start: erg window_start,
        window_count: erg window_count
    );
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => {
            erg closed = region_close(region: ins region);
            return case dat closed {
                Result.Err(_) => 2,
                Result.Ok(_) => 0,
            };
        },
    };
}

verb main() -> Int {
    if size_of[u32]() != 4u64 || align_of[u32]() != 4u64 { return 1; }
    if size_of[Array[u8, 16]]() != 16u64 || align_of[Array[u8, 16]]() != 1u64 { return 2; }
    if size_of[Array[Array[u8, 4], 4]]() != 16u64 || align_of[Array[Array[u8, 4], 4]]() != 1u64 { return 3; }
    if size_of[Pair]() != 8u64 || align_of[Pair]() != 4u64 { return 4; }
    if size_of[Array[Pair, 2]]() != 16u64 || align_of[Array[Pair, 2]]() != 4u64 { return 5; }
    if size_of[Frame]() != 16u64 || align_of[Frame]() != 1u64 { return 6; }

    erg aggregate: Buffer = Buffer[16];
    if open_and_close(backing: dat aggregate) != 0 { return 7; }
    return 0;
}
"#;
    write_fixture(&root, source);
    let status = build_and_run(&root);
    assert_eq!(status.status.code(), Some(0));
}

#[test]
fn hosted_bulk_range_accepts_the_configured_limit() {
    let root = std::env::temp_dir().join(format!("actus-region-range-{}", std::process::id()));
    write_fixture(
        &root,
        r#"import std::region;

verb copy_bulk(ins region: Region[u8], abs source: Buffer) -> Int {
    erg start: u64 = 0u64;
    erg count: u64 = 131072u64;
    erg written = region_write_range(region: ins region, start_index: erg start, element_count: erg count, source: abs source);
    return case dat written {
        Result.Err(_) => 2,
        Result.Ok(value) => {
            if value != count { return 3; }
            erg published = region_publish(region: ins region);
            return case dat published {
                Result.Err(_) => 4,
                Result.Ok(_) => {
                    erg destination: Buffer = Buffer[131072];
                    erg read = region_read_range(region: abs region, start_index: erg start, element_count: erg count, destination: ins destination);
                    return case dat read {
                        Result.Err(_) => 5,
                        Result.Ok(read_count) => if read_count == count { return 0; } else { return 6; },
                    };
                },
            };
        },
    };
}

verb main() -> Int {
    erg backing: Buffer = Buffer[131072];
    erg logical_length: u64 = 131072u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 131072u64;
    erg opened = region_open[u8](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => { erg source: Buffer = Buffer[131072]; return copy_bulk(region: ins region, source: abs source); },
    };
}
"#,
    );
    let execution = build_and_run(&root);
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn aggregate_region_rejects_wrong_buffer_without_dirtying_state() {
    let root =
        std::env::temp_dir().join(format!("actus-region-invalid-buffer-{}", std::process::id()));
    write_fixture(
        &root,
        r#"import std::region;

verb reject_write(ins region: Region[Array[u8, 16]]) -> Int {
    erg source: Buffer = Buffer[15];
    erg index: u64 = 0u64;
    erg written = region_write(region: ins region, index: erg index, source: abs source);
    return case dat written {
        Result.Ok(_) => 2,
        Result.Err(_) => {
            erg dirty = region_dirty(region: abs region);
            return case dat dirty {
                Result.Err(_) => 3,
                Result.Ok(value) => {
                    if value != 0 { return 4; }
                    erg closed = region_close(region: ins region);
                    return case dat closed { Result.Err(_) => 5, Result.Ok(_) => 0, };
                },
            };
        },
    };
}

verb main() -> Int {
    erg backing: Buffer = Buffer[16];
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[Array[u8, 16]](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => { return reject_write(region: ins region); },
    };
}
"#,
    );
    let execution = build_and_run(&root);
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn aggregate_region_rejects_wrong_destination_size() {
    let root = std::env::temp_dir()
        .join(format!("actus-region-invalid-destination-{}", std::process::id()));
    write_fixture(
        &root,
        r#"import std::region;

verb reject_read(ins region: Region[Array[u8, 16]]) -> Int {
    erg destination: Buffer = Buffer[15];
    erg index: u64 = 0u64;
    erg read = region_read(region: abs region, index: erg index, destination: ins destination);
    return case dat read {
        Result.Ok(_) => 2,
        Result.Err(_) => {
            erg closed = region_close(region: ins region);
            return case dat closed { Result.Err(_) => 3, Result.Ok(_) => 0, };
        },
    };
}

verb main() -> Int {
    erg backing: Buffer = Buffer[16];
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[Array[u8, 16]](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => { return reject_read(region: ins region); },
    };
}
"#,
    );
    let execution = build_and_run(&root);
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_bulk_range_rejects_payload_above_the_limit() {
    let root =
        std::env::temp_dir().join(format!("actus-region-range-limit-{}", std::process::id()));
    write_fixture(
        &root,
        r#"import std::region;

verb reject_bulk(ins region: Region[u8]) -> Int {
    erg source: Buffer = Buffer[131073];
    erg start: u64 = 0u64;
    erg count: u64 = 131073u64;
    erg written = region_write_range(region: ins region, start_index: erg start, element_count: erg count, source: abs source);
    return case dat written {
        Result.Ok(_) => 2,
        Result.Err(_) => {
            erg dirty = region_dirty(region: abs region);
            return case dat dirty {
                Result.Err(_) => 3,
                Result.Ok(value) => {
                    if value != 0 { return 4; }
                    erg closed = region_close(region: ins region);
                    return case dat closed { Result.Err(_) => 5, Result.Ok(_) => 0, };
                },
            };
        },
    };
}

verb main() -> Int {
    erg backing: Buffer = Buffer[131073];
    erg logical_length: u64 = 131073u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 131073u64;
    erg opened = region_open[u8](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => { return reject_bulk(region: ins region); },
    };
}
"#,
    );
    let execution = build_and_run(&root);
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

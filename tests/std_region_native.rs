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

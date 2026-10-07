use super::support::{build, project, run};
use object::{Object, ObjectSymbol};
use std::fs;
use std::process::Command;

#[test]
fn nested_runtime_facade_qualifies_sibling_imports() {
    let source = "import feature; verb main() -> Int { return runtime_probe(); }\n";
    let (root, input, output) = project("nested-runtime-sibling-import", source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("enable standard runtime");
    fs::create_dir_all(root.join("src/feature")).expect("create feature module");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "import std::fs; open verb runtime_probe() -> Int { return 0; }\n",
    )
    .expect("write runtime import fixture");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    fs::remove_dir_all(root).expect("remove nested runtime import project");
}

#[test]
fn generic_nested_module_lowers_facade_constant_without_native_binding() {
    let source = "import feature; verb main() -> Int { return inspect[2]() as Int; }\n";
    let (root, input, output) = project("generic-constant-facade", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BUFFER_STRIDE: u16 = 24u16;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "import config; open verb inspect[N: Usize]() -> u16 { return BUFFER_STRIDE + (N as u16); }\n",
    )
    .expect("write generic feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(26));
    fs::remove_dir_all(root).expect("remove generic constant project");
}

#[test]
fn generic_region_facade_object_emits_size_of_with_concrete_element_type() {
    let source = "import std::region; verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg result = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); drop(result); return 0; }\n";
    let (root, input, output) = project("generic-region-object", source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("enable standard runtime");
    let object = output.with_extension("obj");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build generic region object");
    assert!(
        build.status.success(),
        "generic region object build stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(!output.exists(), "object-only build should not create an executable");
    fs::remove_dir_all(root).expect("remove generic region project");
}

#[test]
fn generic_region_lowers_an_imported_packed_element_type() {
    let source = "import feature; import std::region; verb main() -> Int { erg backing: Buffer = Buffer[64]; erg logical_length: u64 = 1048576u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[Minicolumn](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg destination: Buffer = Buffer[64]; erg index: u64 = 0u64; erg read_result = region_read(region: abs region, index: erg index, destination: ins destination); region_close(region: ins region); return case dat read_result { Result.Err(_) => 2, Result.Ok(_) => 0, }; }, }; }\n";
    let (root, input, output) = project("generic-region-imported-pack", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "open pack Minicolumn { erg storage: Array[u8, 64]; layout little; fields { erg marker: u8 at 0; erg threshold: u8 at 8; erg myelination: u8 at 16; erg idle_ticks: u8 at 24; erg flags: u8 at 32; erg payload: u8 at 40; erg layer_depth: u16 at 48; erg coincidence_low: u64 at 64; erg coincidence_high: u64 at 128; erg axons: Array[u32, 8] at 192; erg free_list_link: u32 at 448; erg inhibitory_link: u32 at 480; } }\n",
    )
    .expect("write imported packed type");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("enable standard runtime");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    fs::remove_dir_all(root).expect("remove imported packed Region project");
}

#[test]
fn generic_region_lowers_all_packed_element_operations() {
    let source = r#"import feature;
import std::region;

verb write_publish_cancel(ins region: Region[Minicolumn]) -> Int {
    erg source: Buffer = Buffer[64];
    erg index: u64 = 0u64;
    erg written = region_write(region: ins region, index: erg index, source: abs source);
    return case dat written {
        Result.Err(_) => 1,
        Result.Ok(_) => {
            erg published = region_publish(region: ins region);
            return case dat published {
                Result.Err(_) => 2,
                Result.Ok(_) => {
                    erg canceled = region_cancel(region: ins region);
                    return case dat canceled {
                        Result.Err(_) => 3,
                        Result.Ok(_) => {
                            return 0;
                        },
                    };
                },
            };
        },
    };
}

verb main() -> Int {
    erg backing: Buffer = Buffer[64];
    erg logical_length: u64 = 1048576u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[Minicolumn](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 6,
        Result.Ok(region) => {
            erg write_status: Int = write_publish_cancel(region: ins region);
            if write_status != 0 {
                return write_status;
            }
            erg destination: Buffer = Buffer[64];
            erg index: u64 = 0u64;
            erg read_result = region_read(region: abs region, index: erg index, destination: ins destination);
            erg read_ok: Bool = case dat read_result {
                Result.Err(_) => false,
                Result.Ok(_) => true,
            };
            erg closed = region_close(region: ins region);
            return case dat closed {
                Result.Err(_) => 5,
                Result.Ok(_) => if read_ok { 0 } else { 4 },
            };
        },
    };
}
"#;
    let (root, input, output) = project("generic-region-packed-operations", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "open pack Minicolumn { erg storage: Array[u8, 64]; layout little; fields { erg marker: u8 at 0; erg threshold: u8 at 8; erg myelination: u8 at 16; erg idle_ticks: u8 at 24; erg flags: u8 at 32; erg payload: u8 at 40; erg layer_depth: u16 at 48; erg coincidence_low: u64 at 64; erg coincidence_high: u64 at 128; erg axons: Array[u32, 8] at 192; erg free_list_link: u32 at 448; erg inhibitory_link: u32 at 480; } }\n",
    )
    .expect("write imported packed type");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("enable standard runtime");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    fs::remove_dir_all(root).expect("remove packed Region operations project");
}

#[test]
fn nested_facade_lowers_pack_backed_const_generic_parameters() {
    let source = "import feature; verb main() -> Int { return inspect[256]() as Int; }\n";
    let (root, input, output) = project("pack-backed-generic-facade", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "open pack Minicolumn { erg storage: Array[u8, 64]; layout little; fields { erg marker: u8 at 0; erg threshold: u8 at 8; erg myelination: u8 at 16; erg idle_ticks: u8 at 24; erg flags: u8 at 32; erg payload: u8 at 40; erg layer_depth: u16 at 48; erg coincidence_low: u64 at 64; erg coincidence_high: u64 at 128; erg axons: Array[u32, 8] at 192; erg free_list_link: u32 at 448; erg inhibitory_link: u32 at 480; } } open struct Fabric[N: Usize] { erg columns: Array[Minicolumn, N], } open verb write[N: Usize](ins fabric: Fabric[N]) { fabric.columns[0u32].marker = 41u8; } open verb inspect[N: Usize]() -> u8 { erg fabric: Fabric[N] = Fabric[N] { columns: Array[Minicolumn, N](), }; write[N](fabric: ins fabric); return fabric.columns[0u32].marker; }\n",
    )
    .expect("write pack-backed generic implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove pack-backed generic project");
}

#[test]
fn generic_facade_constant_survives_scalar_predicate_and_aggregate_specialization() {
    let source = "import feature; verb main() -> Int { return inspect[2]() as Int; }\n";
    let (root, input, output) = project("generic-constant-uses", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BUFFER_STRIDE: u16 = 24u16;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import config;

open struct Settings {
    erg stride: u16,
}

open verb inspect[N: Usize]() -> u16 {
    erg scalar: u16 = BUFFER_STRIDE;
    erg settings = Settings { stride: BUFFER_STRIDE, };
    if scalar == BUFFER_STRIDE && (N as u16) == 2u16 {
        return settings.stride + (N as u16);
    }
    return 0u16;
}
"#,
    )
    .expect("write generic feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(26));
    fs::remove_dir_all(root).expect("remove generic constant uses project");
}

#[test]
fn facade_constant_objects_are_deterministic_and_symbol_free() {
    let source = "import feature; verb main() -> Int { return stride() as Int; }\n";
    let (root, input, output) = project("constant-symbol-integrity", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BUFFER_STRIDE: u16 = 24u16;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "import config; open verb stride() -> u16 { return BUFFER_STRIDE; }\n",
    )
    .expect("write feature implementation");

    let first = output.with_extension("first.obj");
    let second = output.with_extension("second.obj");
    for object in [&first, &second] {
        let result = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
            .arg(object)
            .current_dir(&root)
            .output()
            .expect("build deterministic object");
        assert!(
            result.status.success(),
            "object build stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(
        fs::read(&first).expect("read first object"),
        fs::read(&second).expect("read second object")
    );

    for entry in fs::read_dir(&root).expect("read object outputs").flatten() {
        if entry.path().extension().is_none_or(|extension| extension != "obj") {
            continue;
        }
        let bytes = fs::read(entry.path()).expect("read emitted object");
        let object = object::File::parse(bytes.as_slice()).expect("parse emitted object");
        let symbols = object.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
        let unique = symbols.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(symbols.len(), unique.len(), "duplicate symbols in {:?}", entry.path());
        assert!(!symbols.iter().any(|symbol| symbol.contains("BUFFER_STRIDE")));
    }
    fs::remove_dir_all(root).expect("remove symbol integrity project");
}

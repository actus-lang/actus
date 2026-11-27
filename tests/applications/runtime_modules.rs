use super::support::{build, project, run};
use object::{Object, ObjectSymbol};
use std::fs;
use std::process::Command;

#[test]
fn nested_runtime_facade_qualifies_sibling_imports() {
    let source = "import { open feature };\n\"\"\"Entry point that returns the result of runtime_probe as the process exit status.\"\"\"\nverb main() -> Int { return runtime_probe(); }\n";
    let (root, input, output) = project("nested-runtime-sibling-import", source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("enable standard runtime");
    fs::create_dir_all(root.join("src/feature")).expect("create feature module");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import { open std::fs };

"""Return 0 after importing the standard file system module."""
open verb runtime_probe() -> Int { return 0; }
"#,
    )
    .expect("write runtime import fixture");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    fs::remove_dir_all(root).expect("remove nested runtime import project");
}

#[test]
fn generic_nested_module_lowers_facade_constant_without_native_binding() {
    let source = "import { open feature };\n\"\"\"Entry point that returns the Int conversion of inspect instantiated with the generic argument 2.\"\"\"\nverb main() -> Int { return inspect[2]() as Int; }\n";
    let (root, input, output) = project("generic-constant-facade", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(
        root.join("src/config/config.act"),
        r#""""Re-export the configuration constants."""
open values;
"#,
    )
    .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        r#""""The buffer stride, in bytes, that the feature module reads."""
open const BUFFER_STRIDE: u16 = 24u16;
"#,
    )
    .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import { open config };

"""Return the configured stride plus the capacity parameter N."""
open verb inspect[N: Usize]() -> u16 { return BUFFER_STRIDE + (N as u16); }
"#,
    )
    .expect("write generic feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(26));
    fs::remove_dir_all(root).expect("remove generic constant project");
}

#[test]
fn native_lowering_inlines_facade_constant_in_else_if_branch() {
    let source = "import { open feature };\n\"\"\"Entry point that passes the u8 value 17 by ownership to inspect and returns its result.\"\"\"\nverb main() -> Int { erg input: u8 = 17u8; return inspect(value: erg input); }\n";
    let (root, input, output) = project("constant-else-if-lowering", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(
        root.join("src/config/config.act"),
        r#""""Re-export the configuration constants."""
open values;
"#,
    )
    .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        r#""""The value that the else-if branch compares against."""
open const EXPECTED: u8 = 17u8;
"#,
    )
    .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import config;

"""Return 2 for zero, 0 for the configured expected value, and 1 for any other value."""
open verb inspect(erg value: u8) -> Int {
    if value == 0u8 {
        return 2;
    } else if value == EXPECTED {
        return 0;
    }
    return 1;
}
"#,
    )
    .expect("write feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    fs::remove_dir_all(root).expect("remove constant else-if project");
}

#[test]
fn generic_region_facade_object_emits_size_of_with_concrete_element_type() {
    let source = "import { open std::region };\n\"\"\"\ncontract:\npurpose:\n    Open a region of u32 slots over a 4-byte backing buffer and drop the result without using it.\noutputs:\n    0.\nownership:\n    The backing buffer is moved into the region by the open call; the result is dropped.\nerrors:\n    region_open can fail with a RegionError, which is discarded here.\nside_effects:\n    Allocates region storage.\n\"\"\"\nmeta effect(\"allocation\") verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg result = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); drop(result); return 0; }\n";
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
    let source = "import { open feature, open std::region };\n\"\"\"\ncontract:\npurpose:\n    Open a Minicolumn region with a very large logical length, read one element into a destination buffer, then close the region.\noutputs:\n    0 on success, 1 when the region cannot be opened, 2 when the read fails.\nownership:\n    The backing buffer is moved into the region; the destination is borrowed by the read.\nerrors:\n    region_open and region_read errors are mapped to the exit codes 1 and 2.\nside_effects:\n    Allocates region storage and closes the region.\n\"\"\"\nmeta effect(\"allocation\") verb main() -> Int { erg backing: Buffer = Buffer[64]; erg logical_length: u64 = 1048576u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[Minicolumn](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg destination: Buffer = Buffer[64]; erg index: u64 = 0u64; erg read_result = region_read(region: abs region, index: erg index, destination: ins destination); region_close(region: ins region); return case dat read_result { Result.Err(_) => 2, Result.Ok(_) => 0, }; }, }; }\n";
    let (root, input, output) = project("generic-region-imported-pack", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#""""A packed minicolumn record of 64 bytes."""
open pack Minicolumn {
    """The 64-byte storage of the record."""
    erg storage: Array[u8, 64];
    layout little;
    fields {
        """The `marker` field: `u8` at bit offset 0."""
        erg marker: u8 at 0;
        """The `threshold` field: `u8` at bit offset 8."""
        erg threshold: u8 at 8;
        """The `myelination` field: `u8` at bit offset 16."""
        erg myelination: u8 at 16;
        """The `idle_ticks` field: `u8` at bit offset 24."""
        erg idle_ticks: u8 at 24;
        """The `flags` field: `u8` at bit offset 32."""
        erg flags: u8 at 32;
        """The `payload` field: `u8` at bit offset 40."""
        erg payload: u8 at 40;
        """The `layer_depth` field: `u16` at bit offset 48."""
        erg layer_depth: u16 at 48;
        """The `coincidence_low` field: `u64` at bit offset 64."""
        erg coincidence_low: u64 at 64;
        """The `coincidence_high` field: `u64` at bit offset 128."""
        erg coincidence_high: u64 at 128;
        """The `axons` field: `Array[u32, 8]` at bit offset 192."""
        erg axons: Array[u32, 8] at 192;
        """The `free_list_link` field: `u32` at bit offset 448."""
        erg free_list_link: u32 at 448;
        """The `inhibitory_link` field: `u32` at bit offset 480."""
        erg inhibitory_link: u32 at 480;
    }
}
"#,
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
    let source = r#"import { open feature, open std::region };

"""
contract:
purpose:
    Write one Minicolumn into a region, publish the write, then cancel the region, reporting which step failed.
inputs:
    region: exclusive ins loan of Region of Minicolumn values.
outputs:
    0 when write, publish and cancel all succeed, otherwise 1, 2 or 3 for the failing step.
ownership:
    The region is an exclusive ins loan and is mutated; the source buffer is borrowed.
errors:
    region_write, region_publish and region_cancel errors map to the exit codes 1, 2 and 3.
side_effects:
    Mutates the region by writing, publishing and canceling.
"""
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

"""Returns the Minicolumn region it was given, moving ownership from the parameter to the result so the test can check that a packed region survives a forwarding call."""
verb forward_packed_region(dat region: Region[Minicolumn]) -> Region[Minicolumn] {
    return region;
}

"""
contract:
purpose:
    Open a Minicolumn region, forward it through forward_packed_region, run write_publish_cancel on it, then read an element back and close the region.
outputs:
    0 on success, or a nonzero code identifying the failing step.
ownership:
    The region is moved through forward_packed_region and then borrowed by region_read and lent exclusively to region_close; buffers are owned locally.
errors:
    Open, write, read and close errors are mapped to distinct nonzero exit codes.
side_effects:
    Allocates region storage and mutates and closes the region.
"""
meta effect("allocation")
verb main() -> Int {
    erg backing: Buffer = Buffer[64];
    erg logical_length: u64 = 1048576u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    erg opened = region_open[Minicolumn](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count);
    return case dat opened {
        Result.Err(_) => 6,
        Result.Ok(region) => {
            erg forwarded: Region[Minicolumn] = forward_packed_region(region: dat region);
            erg write_status: Int = write_publish_cancel(region: ins forwarded);
            if write_status != 0 {
                return write_status;
            }
            erg destination: Buffer = Buffer[64];
            erg index: u64 = 0u64;
            erg read_result = region_read(region: abs forwarded, index: erg index, destination: ins destination);
            erg read_ok: Bool = case dat read_result {
                Result.Err(_) => false,
                Result.Ok(_) => true,
            };
            erg closed = region_close(region: ins forwarded);
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
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#""""A packed minicolumn record of 64 bytes."""
open pack Minicolumn {
    """The 64-byte storage of the record."""
    erg storage: Array[u8, 64];
    layout little;
    fields {
        """The `marker` field: `u8` at bit offset 0."""
        erg marker: u8 at 0;
        """The `threshold` field: `u8` at bit offset 8."""
        erg threshold: u8 at 8;
        """The `myelination` field: `u8` at bit offset 16."""
        erg myelination: u8 at 16;
        """The `idle_ticks` field: `u8` at bit offset 24."""
        erg idle_ticks: u8 at 24;
        """The `flags` field: `u8` at bit offset 32."""
        erg flags: u8 at 32;
        """The `payload` field: `u8` at bit offset 40."""
        erg payload: u8 at 40;
        """The `layer_depth` field: `u16` at bit offset 48."""
        erg layer_depth: u16 at 48;
        """The `coincidence_low` field: `u64` at bit offset 64."""
        erg coincidence_low: u64 at 64;
        """The `coincidence_high` field: `u64` at bit offset 128."""
        erg coincidence_high: u64 at 128;
        """The `axons` field: `Array[u32, 8]` at bit offset 192."""
        erg axons: Array[u32, 8] at 192;
        """The `free_list_link` field: `u32` at bit offset 448."""
        erg free_list_link: u32 at 448;
        """The `inhibitory_link` field: `u32` at bit offset 480."""
        erg inhibitory_link: u32 at 480;
    }
}
"#,
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
    let source = "import { open feature };\n\"\"\"Entry point that returns the Int conversion of inspect instantiated with the generic argument 256.\"\"\"\nverb main() -> Int { return inspect[256]() as Int; }\n";
    let (root, input, output) = project("pack-backed-generic-facade", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#""""A packed minicolumn record of 64 bytes."""
open pack Minicolumn {
    """The 64-byte storage of the record."""
    erg storage: Array[u8, 64];
    layout little;
    fields {
        """The `marker` field: `u8` at bit offset 0."""
        erg marker: u8 at 0;
        """The `threshold` field: `u8` at bit offset 8."""
        erg threshold: u8 at 8;
        """The `myelination` field: `u8` at bit offset 16."""
        erg myelination: u8 at 16;
        """The `idle_ticks` field: `u8` at bit offset 24."""
        erg idle_ticks: u8 at 24;
        """The `flags` field: `u8` at bit offset 32."""
        erg flags: u8 at 32;
        """The `payload` field: `u8` at bit offset 40."""
        erg payload: u8 at 40;
        """The `layer_depth` field: `u16` at bit offset 48."""
        erg layer_depth: u16 at 48;
        """The `coincidence_low` field: `u64` at bit offset 64."""
        erg coincidence_low: u64 at 64;
        """The `coincidence_high` field: `u64` at bit offset 128."""
        erg coincidence_high: u64 at 128;
        """The `axons` field: `Array[u32, 8]` at bit offset 192."""
        erg axons: Array[u32, 8] at 192;
        """The `free_list_link` field: `u32` at bit offset 448."""
        erg free_list_link: u32 at 448;
        """The `inhibitory_link` field: `u32` at bit offset 480."""
        erg inhibitory_link: u32 at 480;
    }
}

"""A fabric of N minicolumns."""
open struct Fabric[N: Usize] {
    """The minicolumns of the fabric."""
    erg columns: Array[Minicolumn, N],
}

"""Set the marker of the first minicolumn to 41."""
open verb write[N: Usize](ins fabric: Fabric[N]) { fabric.columns[0u32].marker = 41u8; }

"""Build a fabric, set the marker of its first minicolumn through write, and return that marker."""
open verb inspect[N: Usize]() -> u8 { erg fabric: Fabric[N] = Fabric[N] { columns: Array[Minicolumn, N](), }; write[N](fabric: ins fabric); return fabric.columns[0u32].marker; }
"#,
    )
    .expect("write pack-backed generic implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove pack-backed generic project");
}

#[test]
fn generic_region_covers_array_struct_and_aggregate_return_abi() {
    let source = r#"import { open std::region };

struct Pair {
    erg left: u32,
    erg right: u32,
}

"""Returns the generic Region it was given, moving ownership from the parameter to the result."""
verb forward_region[T](dat region: Region[T]) -> Region[T] {
    return region;
}

"""
contract:
purpose:
    Open a one-element region of type T over the given backing buffer.
inputs:
    backing: owned Buffer that becomes the region storage.
outputs:
    Result.Ok with the opened Region of T, or Result.Err with a RegionError.
ownership:
    The backing buffer is moved into the region.
errors:
    Any RegionError from region_open is returned unchanged.
side_effects:
    Allocates region storage.
"""
meta effect("allocation")
verb open_forward[T](dat backing: Buffer) -> Result[Region[T], RegionError] {
    erg logical_length: u64 = 1u64;
    erg window_start: u64 = 0u64;
    erg window_count: u64 = 1u64;
    return region_open[T](
        backing: dat backing,
        logical_length: erg logical_length,
        window_start: erg window_start,
        window_count: erg window_count
    );
}

"""
contract:
purpose:
    Open a region of u16 pairs, forward it, read one element and close it, mapping every failure to an exit code.
outputs:
    0 on success, 1 when the region cannot be opened, 2 when the read fails, 3 when the close fails.
ownership:
    The region is moved through forward_region and then borrowed by region_read and lent exclusively to region_close; buffers are owned locally.
errors:
    Open, read and close errors map to the exit codes 1, 2 and 3.
side_effects:
    Allocates region storage and closes the region.
"""
meta effect("allocation")
verb check_array() -> Int {
    erg backing: Buffer = Buffer[4];
    erg opened = open_forward[Array[u16, 2]](backing: dat backing);
    return case dat opened {
        Result.Err(_) => 1,
        Result.Ok(region) => {
            erg forwarded: Region[Array[u16, 2]] = forward_region(region: dat region);
            erg destination: Buffer = Buffer[4];
            erg index: u64 = 0u64;
            erg read = region_read(region: abs forwarded, index: erg index, destination: ins destination);
            erg closed = region_close(region: ins forwarded);
            erg close_status: Int = case dat closed {
                Result.Err(_) => 3,
                Result.Ok(_) => 0,
            };
            return case dat read {
                Result.Err(_) => 2,
                Result.Ok(_) => close_status,
            };
        },
    };
}

"""
contract:
purpose:
    Open a region of Pair structs, forward it, read one element and close it, mapping every failure to an exit code.
outputs:
    0 on success, 4 when the region cannot be opened, 5 when the read fails, 6 when the close fails.
ownership:
    The region is moved through forward_region and then borrowed by region_read and lent exclusively to region_close; buffers are owned locally.
errors:
    Open, read and close errors map to the exit codes 4, 5 and 6.
side_effects:
    Allocates region storage and closes the region.
"""
meta effect("allocation")
verb check_struct() -> Int {
    erg backing: Buffer = Buffer[8];
    erg opened = open_forward[Pair](backing: dat backing);
    return case dat opened {
        Result.Err(_) => 4,
        Result.Ok(region) => {
            erg forwarded: Region[Pair] = forward_region(region: dat region);
            erg destination: Buffer = Buffer[8];
            erg index: u64 = 0u64;
            erg read = region_read(region: abs forwarded, index: erg index, destination: ins destination);
            erg closed = region_close(region: ins forwarded);
            erg close_status: Int = case dat closed {
                Result.Err(_) => 6,
                Result.Ok(_) => 0,
            };
            return case dat read {
                Result.Err(_) => 5,
                Result.Ok(_) => close_status,
            };
        },
    };
}

"""
contract:
purpose:
    Open a region of 16-byte arrays, forward it, read one element and close it, mapping every failure to an exit code.
outputs:
    0 on success, 7 when the region cannot be opened, 8 when the read fails, 9 when the close fails.
ownership:
    The region is moved through forward_region and then borrowed by region_read and lent exclusively to region_close; buffers are owned locally.
errors:
    Open, read and close errors map to the exit codes 7, 8 and 9.
side_effects:
    Allocates region storage and closes the region.
"""
meta effect("allocation")
verb check_byte_array() -> Int {
    erg backing: Buffer = Buffer[16];
    erg opened = open_forward[Array[u8, 16]](backing: dat backing);
    return case dat opened {
        Result.Err(_) => 7,
        Result.Ok(region) => {
            erg forwarded: Region[Array[u8, 16]] = forward_region(region: dat region);
            erg destination: Buffer = Buffer[16];
            erg index: u64 = 0u64;
            erg read = region_read(region: abs forwarded, index: erg index, destination: ins destination);
            erg closed = region_close(region: ins forwarded);
            erg close_status: Int = case dat closed {
                Result.Err(_) => 9,
                Result.Ok(_) => 0,
            };
            return case dat read {
                Result.Err(_) => 8,
                Result.Ok(_) => close_status,
            };
        },
    };
}

"""
contract:
purpose:
    Run the array, struct and byte-array region checks in order and stop at the first failure.
outputs:
    0 when all three checks pass, otherwise the first nonzero status.
errors:
    The status of the first failing check is returned unchanged.
side_effects:
    Allocates and closes region storage through the three checks.
"""
meta effect("allocation")
verb main() -> Int {
    erg array_status: Int = check_array();
    if array_status != 0 { return array_status; }
    erg struct_status: Int = check_struct();
    if struct_status != 0 { return struct_status; }
    return check_byte_array();
}
"#;
    let (root, input, output) = project("generic-region-array-struct-abi", source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("enable standard runtime");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    fs::remove_dir_all(root).expect("remove aggregate Region ABI project");
}

#[test]
fn generic_facade_constant_survives_scalar_predicate_and_aggregate_specialization() {
    let source = "import { open feature };\n\"\"\"Entry point that returns the Int conversion of inspect instantiated with the generic argument 2.\"\"\"\nverb main() -> Int { return inspect[2]() as Int; }\n";
    let (root, input, output) = project("generic-constant-uses", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(
        root.join("src/config/config.act"),
        r#""""Re-export the configuration constants."""
open values;
"#,
    )
    .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        r#""""The buffer stride, in bytes, that the feature module reads."""
open const BUFFER_STRIDE: u16 = 24u16;
"#,
    )
    .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import config;

"""A holder of the configured stride."""
open struct Settings {
    """The stride copied from the configuration constant."""
    erg stride: u16,
}

"""Return the configured stride plus N when N is 2, and 0 otherwise."""
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
    let source = "import { open feature };\n\"\"\"Entry point that returns the Int conversion of stride, a configuration constant reached through the canonical parent facade.\"\"\"\nverb main() -> Int { return stride() as Int; }\n";
    let (root, input, output) = project("constant-symbol-integrity", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(
        root.join("src/config/config.act"),
        r#""""Re-export the configuration constants."""
open values;
"#,
    )
    .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        r#""""The buffer stride, in bytes, that the feature module reads."""
open const BUFFER_STRIDE: u16 = 24u16;
"#,
    )
    .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature implementation."""
open implementation;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import { open config };

"""Return the configured buffer stride."""
open verb stride() -> u16 { return BUFFER_STRIDE; }
"#,
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

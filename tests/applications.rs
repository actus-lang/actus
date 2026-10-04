use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use object::{Object, ObjectSymbol};

fn copy_standard_library(root: &Path) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let files = [
        "lib.act",
        "io/io.act",
        "io/error.act",
        "io/reader.act",
        "io/writer.act",
        "io/stdout.act",
        "io/stderr.act",
        "io/stdin.act",
        "io/read.act",
        "io/write.act",
        "io/buffered.act",
        "io/cursor.act",
        "io/copy.act",
        "fs/fs.act",
        "fs/file.act",
        "fs/metadata.act",
        "fs/operations.act",
        "fs/options.act",
        "fs/seek.act",
        "path/path.act",
        "path/types.act",
        "path/storage.act",
        "path/error.act",
        "path/components.act",
        "path/posix.act",
        "path/windows.act",
        "path/predicates.act",
        "path/normalize.act",
        "path/builders.act",
    ];
    for relative in files {
        let destination = root.join("src").join(relative);
        fs::create_dir_all(destination.parent().expect("library parent"))
            .expect("library directory");
        fs::copy(source.join(relative), destination).expect("copy standard-library source");
    }
}

fn project(name: &str, source: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("actus-{name}-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create application project");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write application manifest");
    copy_standard_library(&root);
    let input = root.join("src/main.act");
    fs::write(&input, source).expect("write application source");
    (root.clone(), input, root.join("application"))
}

fn build(root: &Path, input: &Path, output: &Path) {
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(root)
        .output()
        .expect("check application");
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "exe", "-o"])
        .arg(output)
        .current_dir(root)
        .output()
        .expect("build application");
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
}

fn run(root: &Path, output: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(output)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run application");
    child
        .stdin
        .take()
        .expect("application stdin")
        .write_all(input)
        .expect("write application input");
    child.wait_with_output().expect("wait for application")
}

#[test]
fn console_application_proves_io_success_and_eof_contracts() {
    let source = include_str!("../examples/console_application.act");
    let (root, input, output) = project("console-application", source);
    build(&root, &input, &output);
    let success = run(&root, &output, b"hello\n");
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(success.stdout, b"hello\n");
    assert_eq!(success.stderr, b"hello\n");
    let eof = run(&root, &output, b"");
    assert_eq!(eof.status.code(), Some(2));
    assert_eq!(eof.stdout, b"");
    assert_eq!(eof.stderr, b"EO\n");
    fs::remove_dir_all(root).expect("remove console project");
}

#[test]
fn file_utility_proves_path_fs_success_and_typed_failure() {
    let source = include_str!("../examples/file_utility.act");
    let (root, input, output) = project("file-utility", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"ACT");
    assert_eq!(execution.stderr, b"");
    assert!(!root.join("actus_192.bin").exists());
    fs::remove_dir_all(root).expect("remove file utility project");
}

#[test]
fn negative_application_proves_missing_file_is_typed() {
    let source = include_str!("fixtures/applications/missing_file.act");
    let (root, input, output) = project("missing-file", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"");
    assert_eq!(execution.stderr, b"");
    fs::remove_dir_all(root).expect("remove negative application project");
}

#[test]
fn module_application_executes_public_wrapper_with_private_implementation() {
    let source = "import math; verb main() -> Int { return add(); }\n";
    let (root, input, output) = project("module-wrapper", source);
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write module facade");
    fs::write(
        root.join("src/math/api.act"),
        "open verb add() -> Int { return hidden(); } verb hidden() -> Int { return 42; }\n",
    )
    .expect("write module implementation");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    assert_eq!(execution.stdout, b"");
    assert_eq!(execution.stderr, b"");
    fs::remove_dir_all(root).expect("remove module project");
}

#[test]
fn package_configuration_module_emits_without_an_anchor_verb() {
    let source = "import config; verb main() -> Int { return LIMIT as Int; }\n";
    let (root, input, output) = project("const-only-config", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const LIMIT: u8 = 30u8;\n")
        .expect("write configuration values");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(30));
    let object = output.with_extension("obj");
    let object_build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build const-only configuration objects");
    assert!(
        object_build.status.success(),
        "object build stderr: {}",
        String::from_utf8_lossy(&object_build.stderr)
    );
    let module_bytes = fs::read(root.join("application.module.actus_mod_6_config.obj"))
        .expect("read const-only configuration object");
    let module_file = object::File::parse(module_bytes.as_slice())
        .expect("parse const-only configuration object");
    assert!(
        !module_file
            .symbols()
            .filter_map(|symbol| symbol.name().ok())
            .any(|symbol| symbol.contains("LIMIT"))
    );
    fs::remove_dir_all(root).expect("remove const-only configuration project");
}

#[test]
fn package_configuration_supports_all_compile_time_consumers() {
    let source = "import config; struct Settings { erg threshold: u8, } struct Storage[N: Usize] { erg values: Array[u8, N], } verb main() -> Int { erg values: Array[u8, 2] = Array[u8, 2](); values[0] = LIMIT; erg settings = Settings { threshold: values[0], }; erg storage: Storage[2] = Storage[2] { values: Array[u8, 2](), }; storage.values[0] = settings.threshold; erg result = storage.values[0] as Int; if ENABLED == true { return result; } return 2; }\n";
    let (root, input, output) = project("config-acceptance", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        "open const LIMIT: u8 = 30u8; open const ENABLED: Bool = true;\n",
    )
    .expect("write configuration values");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(30));
    fs::remove_dir_all(root).expect("remove configuration acceptance project");
}

#[test]
fn imported_module_can_lower_a_configuration_constant() {
    let source = "import worker; verb main() -> Int { return read_magic(); }\n";
    let (root, input, output) = project("config-imported-module", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BRAIN_MAGIC: u32 = 42u32;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/worker")).expect("create worker module");
    fs::write(root.join("src/worker/worker.act"), "open api;\n").expect("write worker facade");
    fs::write(
        root.join("src/worker/api.act"),
        "import config; open verb read_magic() -> Int { return BRAIN_MAGIC as Int; }\n",
    )
    .expect("write worker implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    fs::remove_dir_all(root).expect("remove imported configuration project");
}

#[test]
fn nested_imported_module_can_lower_a_configuration_constant() {
    let source = "import aie; verb main() -> Int { return read_magic(); }\n";
    let (root, input, output) = project("config-nested-imported-module", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BRAIN_MAGIC: u32 = 42u32;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/aie/persistence")).expect("create nested module");
    fs::write(root.join("src/aie/aie.act"), "open persistence;\n").expect("write aie facade");
    fs::write(root.join("src/aie/persistence/persistence.act"), "open serialization;\n")
        .expect("write persistence facade");
    fs::write(
        root.join("src/aie/persistence/serialization.act"),
        "import config; open verb read_magic() -> Int { return BRAIN_MAGIC as Int; }\n",
    )
    .expect("write serialization implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    fs::remove_dir_all(root).expect("remove nested configuration project");
}

#[test]
fn canonical_parent_facade_exposes_nested_configuration_constant() {
    let source = "import feature; verb main() -> Int { return stride() as Int; }\n";
    let (root, input, output) = project("canonical-constant-facade", source);
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

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(24));
    fs::remove_dir_all(root).expect("remove canonical constant project");
}

#[test]
fn nested_facade_constant_has_object_and_executable_parity() {
    let source = "import feature; verb main() -> Int { return inspect() as Int; }\n";
    let (root, input, output) = project("constant-native-parity", source);
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

open verb inspect() -> u16 {
    erg settings = Settings { stride: BUFFER_STRIDE, };
    if BUFFER_STRIDE == 24u16 {
        return settings.stride;
    }
    return 0u16;
}
"#,
    )
    .expect("write feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(24));

    let object = output.with_extension("obj");
    let object_build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build native objects");
    assert!(
        object_build.status.success(),
        "object build stderr: {}",
        String::from_utf8_lossy(&object_build.stderr)
    );
    let feature_object = fs::read_dir(&root)
        .expect("read project outputs")
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_name().to_string_lossy().contains("feature")
                && entry.path().extension().is_some_and(|extension| extension == "obj")
        })
        .expect("feature object");
    let bytes = fs::read(feature_object.path()).expect("read feature object");
    let object_file = object::File::parse(bytes.as_slice()).expect("parse feature object");
    assert!(
        !object_file
            .symbols()
            .filter_map(|symbol| symbol.name().ok())
            .any(|symbol| symbol.contains("BUFFER_STRIDE"))
    );
    fs::remove_dir_all(root).expect("remove native parity project");
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

#[test]
fn nested_snapshot_module_can_lower_facade_constants_in_struct_and_predicate() {
    let source = "import aie; verb main() -> Int { return snapshot_is_valid(); }\n";
    let (root, input, output) = project("config-snapshot-module", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BRAIN_MAGIC: u32 = 42u32;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/aie/persistence")).expect("create nested module");
    fs::write(root.join("src/aie/aie.act"), "open persistence;\n").expect("write aie facade");
    fs::write(root.join("src/aie/persistence/persistence.act"), "open serialization;\n")
        .expect("write persistence facade");
    fs::write(
        root.join("src/aie/persistence/serialization.act"),
        r#"import config;

open struct Snapshot {
    erg magic: u32,
}

open verb snapshot_is_valid() -> Int {
    erg snapshot = Snapshot {
        magic: BRAIN_MAGIC,
    };
    return case snapshot.magic == BRAIN_MAGIC {
        true => 42,
        _ => snapshot.magic as Int,
    };
}
"#,
    )
    .expect("write serialization implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    fs::remove_dir_all(root).expect("remove snapshot configuration project");
}

#[test]
fn nested_facade_constants_lower_inside_indexed_snapshot_loops() {
    let source = "import feature; verb main() -> Int { return emit(); }\n";
    let (root, input, output) = project("constant-indexed-loop", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        "open const STRIDE: u16 = 24u16; open const COUNT: u32 = 4u32;\n",
    )
    .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import config;

open verb emit() -> Int {
    erg bytes: Array[u8, 24] = Array[u8, 24]();
    erg index: u32 = 0u32;
    loop {
        if index >= COUNT {
            break;
        }
        bytes[index * (STRIDE as u32) / (STRIDE as u32)] = index as u8;
        index += 1u32;
    }
    return bytes[3u32] as Int + STRIDE as Int;
}
"#,
    )
    .expect("write feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(27));
    fs::remove_dir_all(root).expect("remove indexed constant project");
}

#[test]
fn module_wrapper_preserves_runtime_c_abi_across_object_boundaries() {
    let source = "import output; verb main() -> Int { return emit(); }\n";
    let (root, input, output) = project("module-runtime-bridge", source);
    fs::create_dir_all(root.join("src/output")).expect("create output module directory");
    fs::write(root.join("src/output/output.act"), "open api;\n").expect("write output facade");
    fs::write(
        root.join("src/output/api.act"),
        "open unsafe extern \"C\" verb actus_print_int(erg value: Int) -> Int; open verb emit() -> Int { erg value = 79; actus_print_int(value: value); return 0; }\n",
    )
    .expect("write output implementation");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"79\n");
    assert_eq!(execution.stderr, b"");
    fs::remove_dir_all(root).expect("remove runtime bridge project");
}

#[cfg(unix)]
#[test]
fn unresolved_external_bridge_fails_during_multi_object_link() {
    let source = "import missing; verb main() -> Int { return invoke(); }\n";
    let (root, input, output) = project("unresolved-bridge", source);
    fs::create_dir_all(root.join("src/missing")).expect("create missing module directory");
    fs::write(root.join("src/missing/missing.act"), "open api;\n").expect("write missing facade");
    fs::write(
        root.join("src/missing/api.act"),
        "open unsafe extern \"C\" verb absent_bridge() -> Int; open verb invoke() -> Int { return absent_bridge(); }\n",
    )
    .expect("write unresolved bridge implementation");
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(&root)
        .output()
        .expect("check unresolved bridge application");
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));
    let build_result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&root)
        .output()
        .expect("build unresolved bridge application");
    assert!(!build_result.status.success());
    let diagnostics = String::from_utf8_lossy(&build_result.stderr);
    assert!(diagnostics.contains("absent_bridge"), "diagnostics: {diagnostics}");
    fs::remove_dir_all(root).expect("remove unresolved bridge project");
}

#[test]
fn freestanding_multi_object_build_keeps_configured_entry_and_module_symbols() {
    let source = "import math; verb boot() -> Int { return add(); }\n";
    let (root, input, output) = project("freestanding-objects", source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\nentry = \"boot\"\n\n[build]\nentry_contract = \"freestanding\"\n",
    )
    .expect("write freestanding manifest");
    fs::create_dir_all(root.join("src/math")).expect("create math module directory");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write math facade");
    fs::write(root.join("src/math/api.act"), "open verb add() -> Int { return 7; }\n")
        .expect("write math implementation");
    let object = output.with_extension("obj");
    let build_result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build freestanding object set");
    assert!(
        build_result.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&build_result.stderr)
    );
    let root_bytes = fs::read(&object).expect("read root object");
    let root_file = object::File::parse(root_bytes.as_slice()).expect("parse root object");
    let root_symbols =
        root_file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(root_symbols.iter().any(|symbol| symbol_matches(symbol, "boot")));
    let module = root.join("application.module.actus_mod_4_math.obj");
    let module_bytes = fs::read(&module).expect("read module object");
    let module_file = object::File::parse(module_bytes.as_slice()).expect("parse module object");
    let module_symbols =
        module_file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(
        module_symbols.iter().any(|symbol| symbol_matches(symbol, "actus_mod_4_math__verb_add"))
    );
    fs::remove_dir_all(root).expect("remove freestanding object project");
}

#[test]
fn object_build_emits_one_root_and_one_imported_module_object() {
    let source = "import math; verb main() -> Int { return add(); }\n";
    let (root, input, output) = project("module-objects", source);
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write module facade");
    fs::write(
        root.join("src/math/api.act"),
        "open verb add() -> Int { return hidden(); } verb hidden() -> Int { return 42; }\n",
    )
    .expect("write module implementation");
    let object = output.with_extension("obj");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build module objects");
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
    assert!(object.exists());
    assert!(root.join("application.module.actus_mod_4_math.obj").exists());
    fs::remove_dir_all(root).expect("remove module object project");
}

#[test]
fn module_application_rejects_private_bridge_before_codegen() {
    let source = "import runtime; verb main() -> Int { return private_bridge(); }\n";
    let (root, input, _output) = project("private-bridge", source);
    fs::create_dir_all(root.join("src/runtime")).expect("create runtime module directory");
    fs::write(root.join("src/runtime/runtime.act"), "open api;\n").expect("write runtime facade");
    fs::write(
        root.join("src/runtime/api.act"),
        "open verb read() -> Int { return private_bridge(); } unsafe extern \"C\" verb private_bridge() -> Int;\n",
    )
    .expect("write runtime implementation");
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(&root)
        .output()
        .expect("check private bridge application");
    assert!(!check.status.success());
    let diagnostics = String::from_utf8_lossy(&check.stderr);
    assert!(diagnostics.contains("E1109"), "diagnostics: {diagnostics}");
    fs::remove_dir_all(root).expect("remove private bridge project");
}

fn symbol_matches(actual: &str, expected: &str) -> bool {
    actual == expected || actual.strip_prefix('_') == Some(expected)
}

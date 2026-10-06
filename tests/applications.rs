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

fn generic_result_facade_project(import_path: &str) -> (PathBuf, PathBuf, PathBuf) {
    let name = format!("generic-result-{}", import_path.replace("::", "-"));
    generic_result_facade_project_named(import_path, &name)
}

fn generic_result_facade_project_named(
    import_path: &str,
    name: &str,
) -> (PathBuf, PathBuf, PathBuf) {
    let source = format!("import {import_path}; verb main() -> Int {{ return 0; }}\n");
    let (root, input, output) = project(name, &source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("enable hosted standard library");
    fs::create_dir_all(root.join("src/feature/runtime")).expect("create nested facade");
    fs::write(root.join("src/feature/feature.act"), "open runtime;\n")
        .expect("write feature facade");
    fs::write(root.join("src/feature/runtime/runtime.act"), "open api; open save;\n")
        .expect("write runtime facade");
    fs::write(
        root.join("src/feature/runtime/api.act"),
        "import std::io; import std::fs; import std::path; open io; open fs; open path; open verb write_stage(abs staged: Path, abs payload: Buffer) -> Result[Int, IoError] { return write_file(path: abs staged, contents: abs payload); } open verb publish_stage(abs staged: Path, abs active: Path) -> Result[Int, IoError] { erg loaded = read_to_bytes(path: abs staged); return case dat loaded { Result.Err(error) => Err(error), Result.Ok(bytes) => { drop(bytes); return rename(from: abs staged, to: abs active); }, }; }\n",
    )
    .expect("write result producer");
    fs::write(
        root.join("src/feature/runtime/flush.act"),
        "import std::io; import std::path; open io; open path; open api; open verb flush_stage(abs staged: Path, abs active: Path, abs payload: Buffer) -> Result[Int, IoError] { erg written = write_stage(staged: abs staged, payload: abs payload); return case dat written { Result.Err(error) => Err(error), Result.Ok(_) => publish_stage(staged: abs staged, active: abs active), }; }\n",
    )
    .expect("write result flusher");
    fs::write(
        root.join("src/feature/runtime/save.act"),
        "import std::io; import std::path; open io; open path; open flush; open struct State { erg value: Int, } verb complete(ins state: State, dat flushed: Result[Int, IoError]) -> Result[Int, IoError] { return case dat flushed { Result.Err(error) => Err(error), Result.Ok(bytes) => { state.value = bytes; return Ok(bytes); }, }; } open verb forward(ins state: State, abs staged: Path, abs active: Path, abs payload: Buffer) -> Result[Int, IoError] { erg flushed = flush_stage(staged: abs staged, active: abs active, payload: abs payload); return complete(state: ins state, flushed: dat flushed); }\n",
    )
    .expect("write result forwarder");

    (root, input, output)
}

#[test]
fn generic_result_nested_facade_baseline_builds_natively() {
    let (root, input, output) = generic_result_facade_project("feature");
    build(&root, &input, &output);
    fs::remove_dir_all(root).expect("remove generic result baseline project");
}

#[test]
fn generic_result_direct_child_facade_baseline_builds_natively() {
    let (root, input, output) = generic_result_facade_project("feature::runtime");
    build(&root, &input, &output);
    fs::remove_dir_all(root).expect("remove generic result direct facade project");
}

#[test]
fn generic_result_facade_keeps_object_and_executable_identity_in_sync() {
    let (root, input, output) =
        generic_result_facade_project_named("feature", "generic-result-object-parity");
    build(&root, &input, &output);

    let object = output.with_extension("obj");
    let object_build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build generic result object");
    assert!(
        object_build.status.success(),
        "object build stderr: {}",
        String::from_utf8_lossy(&object_build.stderr)
    );
    let bytes = fs::read(&object).expect("read generic result object");
    object::File::parse(bytes.as_slice()).expect("generic result object should be valid");
    assert!(output.is_file(), "executable build should remain available");

    fs::remove_dir_all(root).expect("remove generic result parity project");
}

#[test]
fn nested_generic_option_and_result_returns_preserve_native_discriminants() {
    let source = r#"
struct Storage[N: Usize] {
    erg value: u32,
}

enum Failure {
    Failed,
}

verb make_option[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    return Option[u32].Some(storage.value);
}

verb forward_option[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    erg produced = make_option(storage: ins storage);
    return case dat produced {
        Option.Some(value) => Option[u32].Some(value),
        Option.None => Option[u32].None,
    };
}

verb make_result[N: Usize](ins storage: Storage[N]) -> Result[u32, Failure] {
    if storage.value == 41u32 {
        return Result[u32, Failure].Ok(storage.value);
    }
    return Result[u32, Failure].Err(Failure.Failed);
}

verb plain_option() -> Option[u32] {
    return Option[u32].Some(41u32);
}

verb plain_result() -> Result[u32, Failure] {
    return Result[u32, Failure].Ok(41u32);
}

verb forward_plain_option[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    erg produced = plain_option();
    return case dat produced {
        Option.Some(value) => Option[u32].Some(value + storage.value - 41u32),
        Option.None => Option[u32].None,
    };
}

verb forward_plain_result[N: Usize](ins storage: Storage[N]) -> Result[u32, Failure] {
    erg produced = plain_result();
    return case dat produced {
        Result.Ok(value) => Result[u32, Failure].Ok(value + storage.value - 41u32),
        Result.Err(error) => Result[u32, Failure].Err(error),
    };
}

verb forward_result[N: Usize](ins storage: Storage[N]) -> Result[u32, Failure] {
    erg produced = make_result(storage: ins storage);
    return case dat produced {
        Result.Ok(value) => Result[u32, Failure].Ok(value),
        Result.Err(error) => Result[u32, Failure].Err(error),
    };
}

verb main() -> Int {
    erg storage: Storage[4] = Storage[4] { value: 41u32, };
    erg option = forward_option(storage: ins storage);
    erg option_value: u32 = case dat option {
        Option.Some(value) => value,
        Option.None => 0u32,
    };
    erg result = forward_result(storage: ins storage);
    erg result_value: u32 = case dat result {
        Result.Ok(value) => value,
        Result.Err(_) => 0u32,
    };
    erg plain_option_result = forward_plain_option(storage: ins storage);
    erg plain_option_value: u32 = case dat plain_option_result {
        Option.Some(value) => value,
        Option.None => 0u32,
    };
    erg plain_result_result = forward_plain_result(storage: ins storage);
    erg plain_result_value: u32 = case dat plain_result_result {
        Result.Ok(value) => value,
        Result.Err(_) => 0u32,
    };
    return option_value as Int + result_value as Int + plain_option_value as Int + plain_result_value as Int;
}
"#;
    let (root, input, output) = project("nested-generic-enum-discriminants", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(164));
    fs::remove_dir_all(root).expect("remove nested generic enum project");
}

#[test]
fn nested_generic_option_result_chain_preserves_discriminants_natively() {
    let source = r#"
struct Storage {
    erg value: u32,
}

struct Cache {
    erg slot: u8,
}

enum Failure {
    Failed,
}

verb reserve() -> Result[u8, Failure] {
    return Result[u8, Failure].Ok(0u8);
}

verb create_column() -> Option[u32] {
    return Option[u32].Some(7u32);
}

verb finish[N: Usize](ins storage: Storage, ins cache: Cache, erg slot: u8) -> Result[u32, Failure] {
    erg created = create_column();
    erg result: Result[u32, Failure] = Result[u32, Failure].Err(Failure.Failed);
    case dat created {
        Option.None => {},
        Option.Some(index) => {
            cache.slot = slot;
            result = Result[u32, Failure].Ok(index + storage.value);
        },
    };
    return result;
}

verb complete[N: Usize](ins storage: Storage, ins cache: Cache) -> Result[u32, Failure] {
    erg prepared = reserve();
    return case dat prepared {
        Result.Err(error) => Result[u32, Failure].Err(error),
        Result.Ok(slot) => finish[N](storage: ins storage, cache: ins cache, slot: erg slot),
    };
}

verb main() -> Int {
    erg storage: Storage = Storage { value: 41u32, };
    erg cache: Cache = Cache { slot: 0u8, };
    erg result = complete[4](storage: ins storage, cache: ins cache);
    return case dat result {
        Result.Ok(value) => value as Int,
        Result.Err(_) => 0,
    };
}
"#;
    let (root, input, output) = project("nested-generic-option-result-chain", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(48));
    fs::remove_dir_all(root).expect("remove nested generic chain project");
}

#[test]
fn nested_generic_result_preserves_aggregate_payload_ownership_natively() {
    let source = r#"
struct Payload {
    erg value: u32,
}

enum Failure {
    Failed,
}

verb produce[N: Usize]() -> Result[Payload, Failure] {
    return Result[Payload, Failure].Ok(Payload { value: 41u32, });
}

verb forward[N: Usize]() -> Result[Payload, Failure] {
    erg result = produce[N]();
    return case dat result {
        Result.Ok(payload) => Result[Payload, Failure].Ok(payload),
        Result.Err(error) => Result[Payload, Failure].Err(error),
    };
}

verb main() -> Int {
    erg result = forward[4]();
    return case dat result {
        Result.Ok(payload) => payload.value as Int,
        Result.Err(_) => 0,
    };
}
"#;
    let (root, input, output) = project("nested-generic-aggregate-enum", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove nested aggregate enum project");
}

#[test]
fn nested_facade_generic_result_preserves_aggregate_payload_natively() {
    let source = "import feature; verb main() -> Int { erg result = forward[4](); return case dat result { Result.Ok(payload) => payload.value as Int, Result.Err(_) => 0, }; }\n";
    let (root, input, output) = project("nested-facade-aggregate-enum", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature facade");
    fs::write(root.join("src/feature/feature.act"), "open api;\n").expect("write feature facade");
    fs::write(
        root.join("src/feature/api.act"),
        "open struct Payload { erg value: u32, } open enum Failure { Failed, } open verb produce[N: Usize]() -> Result[Payload, Failure] { return Result[Payload, Failure].Ok(Payload { value: 41u32, }); } open verb forward[N: Usize]() -> Result[Payload, Failure] { erg result = produce[N](); return case dat result { Result.Ok(payload) => Result[Payload, Failure].Ok(payload), Result.Err(error) => Result[Payload, Failure].Err(error), }; }\n",
    )
    .expect("write nested facade implementation");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove nested facade aggregate project");
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
fn imported_io_copy_does_not_capture_scalar_copy_intrinsic() {
    let source = r#"import io;

verb scalar_copy() -> Int {
    erg value: u32 = 41u32;
    erg reused: u32 = copy(value: abs value);
    return reused as Int;
}

verb main() -> Int {
    return scalar_copy();
}
"#;
    let (root, input, output) = project("imported-io-copy-intrinsic", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove imported io copy project");
}

#[test]
fn nested_facade_keeps_scalar_copy_intrinsic_separate_from_imported_io_copy() {
    let source = "import feature; verb main() -> Int { return scalar_copy(); }\n";
    let (root, input, output) = project("nested-imported-io-copy-intrinsic", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature module");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import io;

open struct Storage[N: Usize] {
    erg value: u32,
}

open verb configure(ins storage: Storage[64], erg index: u32) -> u32 {
    storage.value = index;
    return index;
}

open verb produce[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    return Option[u32].Some(41u32);
}

open verb scalar_copy() -> Int {
    erg storage: Storage[64] = Storage[64] { value: 0u32, };
    erg created: Option[u32] = produce(storage: ins storage);
    erg selected: u32 = case dat created {
        Option.Some(index) => configure(storage: ins storage, index: erg index),
        Option.None => 0u32,
    };
    erg reused: u32 = copy(value: abs selected);
    return reused as Int;
}
"#,
    )
    .expect("write feature implementation");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove nested imported io copy project");
}

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

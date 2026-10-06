use super::support::{build, project, run};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
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
fn nested_result_case_with_aggregate_payload_preserves_native_identity() {
    let source = r#"
struct Storage[N: Usize] {
    erg value: u64,
}

struct TokenCode {
    erg mask_low: u64,
    erg mask_high: u64,
}

enum Failure {
    Invalid,
}

verb validate[N: Usize](ins storage: Storage[N]) -> Result[u64, Failure] {
    return Result[u64, Failure].Ok(storage.value);
}

verb encode[N: Usize](ins storage: Storage[N]) -> Result[TokenCode, Failure] {
    erg admission = validate(storage: ins storage);
    return case dat admission {
        Result.Err(error) => Result[TokenCode, Failure].Err(error),
        Result.Ok(value) => Result[TokenCode, Failure].Ok(TokenCode { mask_low: value, mask_high: 0u64, }),
    };
}

verb main() -> Int {
    erg storage: Storage[4] = Storage[4] { value: 41u64, };
    erg result = encode(storage: ins storage);
    return case dat result {
        Result.Ok(code) => code.mask_low as Int,
        Result.Err(_) => 0,
    };
}
"#;
    let (root, input, output) = project("nested-result-aggregate-case-identity", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove nested result aggregate case project");
}

#[test]
fn result_case_block_tail_preserves_native_identity() {
    let source = r#"
struct TokenCode {
    erg mask_low: u64,
}

enum Failure {
    Invalid,
}

verb encode() -> Result[TokenCode, Failure] {
    erg admission: Result[u64, Failure] = Result[u64, Failure].Ok(41u64);
    return case dat admission {
        Result.Err(error) => {
            Result[TokenCode, Failure].Err(error);
        },
        Result.Ok(value) => {
            Result[TokenCode, Failure].Ok(TokenCode { mask_low: value, });
        },
    };
}

verb main() -> Int {
    erg result = encode();
    return case dat result {
        Result.Ok(code) => code.mask_low as Int,
        Result.Err(_) => 0,
    };
}
"#;
    let (root, input, output) = project("result-case-block-tail", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove result case block tail project");
}

#[test]
fn nested_facade_result_case_with_aggregate_payload_preserves_native_identity() {
    let source = "import feature; verb main() -> Int { return run(); }\n";
    let (root, input, output) = project("nested-facade-result-aggregate-case", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open api;\n").expect("write feature facade");
    fs::write(root.join("src/feature/api.act"), "open types; open encoding; open verb run() -> Int { erg result = encode(); return case dat result { Result.Ok(code) => code.mask_low as Int, Result.Err(_) => 0, }; }\n")
        .expect("write feature api");
    fs::write(
        root.join("src/feature/types.act"),
        "open struct TokenCode { erg mask_low: u64, erg mask_high: u64, } open enum Failure { Invalid, }\n",
    )
    .expect("write feature types");
    fs::write(
        root.join("src/feature/encoding.act"),
        "open verb validate() -> Result[u64, Failure] { return Result[u64, Failure].Ok(41u64); } open verb encode() -> Result[TokenCode, Failure] { erg admission = validate(); return case dat admission { Result.Err(error) => Result[TokenCode, Failure].Err(error), Result.Ok(value) => Result[TokenCode, Failure].Ok(TokenCode { mask_low: value, mask_high: 0u64, }), }; }\n",
    )
    .expect("write feature encoding");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove nested facade result aggregate case project");
}

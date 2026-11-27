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
    let source = format!(
        "import {{ open {import_path} }};\n\"\"\"Entry point of an otherwise empty generic-result facade project; it returns 0.\"\"\"\nverb main() -> Int {{ return 0; }}\n"
    );
    let (root, input, output) = project(name, &source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("enable hosted standard library");
    fs::create_dir_all(root.join("src/feature/runtime")).expect("create nested facade");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the runtime module."""
open runtime;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/runtime/runtime.act"),
        r#""""Re-export the staging verbs."""
open api;

"""Re-export the forwarding verb and its state."""
open save;
"#,
    )
    .expect("write runtime facade");
    fs::write(
        root.join("src/feature/runtime/api.act"),
        r#"import std::io;
import std::fs;
import std::path;

"""Re-export the standard input and output module."""
open io;

"""Re-export the standard file system module."""
open fs;

"""Re-export the standard path module."""
open path;

"""Write the payload to the staged path and return the result of the write."""
meta effect("filesystem", "blocking")
open verb write_stage(abs staged: Path, abs payload: Buffer) -> Result[Int, IoError] { return write_file(path: abs staged, contents: abs payload); }

"""Read the staged file back and rename it to the active path, returning the error of the first step that fails."""
meta effect("filesystem", "blocking")
open verb publish_stage(abs staged: Path, abs active: Path) -> Result[Int, IoError] { erg loaded = read_to_bytes(path: abs staged); return case dat loaded { Result.Err(error) => Err(error), Result.Ok(bytes) => { drop(bytes); return rename(from: abs staged, to: abs active); }, }; }
"#
,
    )
    .expect("write result producer");
    fs::write(
        root.join("src/feature/runtime/flush.act"),
        r#"import std::io;
import std::path;

"""Re-export the standard input and output module."""
open io;

"""Re-export the standard path module."""
open path;

"""Re-export the staging verbs."""
open api;

"""Write the payload to the staged path, then publish it to the active path."""
meta effect("filesystem", "blocking")
open verb flush_stage(abs staged: Path, abs active: Path, abs payload: Buffer) -> Result[Int, IoError] { erg written = write_stage(staged: abs staged, payload: abs payload); return case dat written { Result.Err(error) => Err(error), Result.Ok(_) => publish_stage(staged: abs staged, active: abs active), }; }
"#
,
    )
    .expect("write result flusher");
    fs::write(
        root.join("src/feature/runtime/save.act"),
        r#"import std::io;
import std::path;

"""Re-export the standard input and output module."""
open io;

"""Re-export the standard path module."""
open path;

"""Re-export the flushing verb."""
open flush;

"""A holder of the number of bytes that the last flush wrote."""
open struct State {
    """The number of bytes written by the last flush."""
    erg value: Int,
}

"""Record the written byte count in the state and return it, or return the error of the flush."""
verb complete(ins state: State, dat flushed: Result[Int, IoError]) -> Result[Int, IoError] { return case dat flushed { Result.Err(error) => Err(error), Result.Ok(bytes) => { state.value = bytes; return Ok(bytes); }, }; }

"""Flush the payload from the staged path to the active path and record the written byte count in the state."""
meta effect("filesystem", "blocking")
open verb forward(ins state: State, abs staged: Path, abs active: Path, abs payload: Buffer) -> Result[Int, IoError] { erg flushed = flush_stage(staged: abs staged, active: abs active, payload: abs payload); return complete(state: ins state, flushed: dat flushed); }
"#
,
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

"""
contract:
purpose:
    Wrap the value field of the storage in Option.Some.
inputs:
    storage: exclusive ins loan of Storage of capacity N whose value is a u32.
outputs:
    Option.Some holding the storage value.
ownership:
    The storage is an exclusive ins loan and is only read.
"""
verb make_option[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    return Option[u32].Some(storage.value);
}

"""
contract:
purpose:
    Forward the Option produced by make_option unchanged, rebuilding each variant by case.
inputs:
    storage: exclusive ins loan of Storage of capacity N.
outputs:
    Option.Some with the same payload, or Option.None.
ownership:
    The storage stays an exclusive ins loan and is passed on to make_option.
"""
verb forward_option[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    erg produced = make_option(storage: ins storage);
    return case dat produced {
        Option.Some(value) => Option[u32].Some(value),
        Option.None => Option[u32].None,
    };
}

"""
contract:
purpose:
    Produce Ok with the storage value when it equals 41, otherwise a Failure.
inputs:
    storage: exclusive ins loan of Storage of capacity N whose value is a u32.
outputs:
    Result.Ok with the value, or Result.Err with Failure.Failed.
ownership:
    The storage is an exclusive ins loan and is only read.
errors:
    Failure.Failed when the stored value is not 41.
"""
verb make_result[N: Usize](ins storage: Storage[N]) -> Result[u32, Failure] {
    if storage.value == 41u32 {
        return Result[u32, Failure].Ok(storage.value);
    }
    return Result[u32, Failure].Err(Failure.Failed);
}

"""Returns Option.Some holding the u32 constant 41."""
verb plain_option() -> Option[u32] {
    return Option[u32].Some(41u32);
}

"""Returns Result.Ok holding the u32 constant 41."""
verb plain_result() -> Result[u32, Failure] {
    return Result[u32, Failure].Ok(41u32);
}

"""
contract:
purpose:
    Forward the Option from plain_option after adding the storage value minus 41 to its payload.
inputs:
    storage: exclusive ins loan of Storage of capacity N.
outputs:
    Option.Some with the adjusted payload, or Option.None.
ownership:
    The storage is an exclusive ins loan and is only read.
"""
verb forward_plain_option[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    erg produced = plain_option();
    return case dat produced {
        Option.Some(value) => Option[u32].Some(value + storage.value - 41u32),
        Option.None => Option[u32].None,
    };
}

"""
contract:
purpose:
    Forward the Result from plain_result after adding the storage value minus 41 to an Ok payload.
inputs:
    storage: exclusive ins loan of Storage of capacity N.
outputs:
    Result.Ok with the adjusted payload, or the original error.
ownership:
    The storage is an exclusive ins loan and is only read.
errors:
    Any error produced by plain_result is passed through.
"""
verb forward_plain_result[N: Usize](ins storage: Storage[N]) -> Result[u32, Failure] {
    erg produced = plain_result();
    return case dat produced {
        Result.Ok(value) => Result[u32, Failure].Ok(value + storage.value - 41u32),
        Result.Err(error) => Result[u32, Failure].Err(error),
    };
}

"""
contract:
purpose:
    Forward the Result produced by make_result unchanged, rebuilding each variant by case.
inputs:
    storage: exclusive ins loan of Storage of capacity N.
outputs:
    Result.Ok with the same payload, or the same Failure.
ownership:
    The storage stays an exclusive ins loan and is passed on to make_result.
errors:
    Failure values from make_result are passed through.
"""
verb forward_result[N: Usize](ins storage: Storage[N]) -> Result[u32, Failure] {
    erg produced = make_result(storage: ins storage);
    return case dat produced {
        Result.Ok(value) => Result[u32, Failure].Ok(value),
        Result.Err(error) => Result[u32, Failure].Err(error),
    };
}

"""Entry point that builds a storage holding 41, runs all four forwarding verbs on it, unwraps each Option or Result with a case, and returns the sum of the four payloads as Int."""
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

"""Returns Result.Ok holding the u8 slot number 0, standing in for a successful slot reservation."""
verb reserve() -> Result[u8, Failure] {
    return Result[u8, Failure].Ok(0u8);
}

"""Returns Option.Some holding the u32 column index 7, standing in for a successful column creation."""
verb create_column() -> Option[u32] {
    return Option[u32].Some(7u32);
}

"""
contract:
purpose:
    Create a column and, when one exists, record the reserved slot in the cache and return the column index plus the storage value.
inputs:
    storage: exclusive ins loan of Storage; cache: exclusive ins loan of Cache whose slot is written; slot: owned u8 slot number.
outputs:
    Result.Ok with the column index plus the storage value, or Result.Err with Failure.Failed.
ownership:
    The cache is an exclusive ins loan and is mutated; the slot is consumed.
errors:
    Failure.Failed when create_column yields no column.
side_effects:
    Writes the slot into the cache.
"""
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

"""
contract:
purpose:
    Reserve a slot and, on success, finish the operation with it.
inputs:
    storage: exclusive ins loan of Storage; cache: exclusive ins loan of Cache.
outputs:
    The Result of finish, or the error from reserve.
ownership:
    Storage and cache stay exclusive ins loans and are passed on to finish.
errors:
    The error from reserve is passed through.
side_effects:
    finish records the slot in the cache.
"""
verb complete[N: Usize](ins storage: Storage, ins cache: Cache) -> Result[u32, Failure] {
    erg prepared = reserve();
    return case dat prepared {
        Result.Err(error) => Result[u32, Failure].Err(error),
        Result.Ok(slot) => finish[N](storage: ins storage, cache: ins cache, slot: erg slot),
    };
}

"""Entry point that builds a storage holding 41 and an empty cache, runs complete with capacity 4, and returns the Ok value as Int or 0 on error."""
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

"""
contract:
purpose:
    Produce an Ok result owning a Payload with the value 41.
outputs:
    Result.Ok with a new Payload, or Result.Err with Failure.
ownership:
    The returned Payload is owned by the caller.
"""
verb produce[N: Usize]() -> Result[Payload, Failure] {
    return Result[Payload, Failure].Ok(Payload { value: 41u32, });
}

"""
contract:
purpose:
    Forward the Result from produce, moving the Payload through the case unchanged.
outputs:
    Result.Ok owning the same Payload, or the same Failure.
ownership:
    The Payload moves from the produced result into the returned one.
errors:
    Failure values from produce are passed through.
"""
verb forward[N: Usize]() -> Result[Payload, Failure] {
    erg result = produce[N]();
    return case dat result {
        Result.Ok(payload) => Result[Payload, Failure].Ok(payload),
        Result.Err(error) => Result[Payload, Failure].Err(error),
    };
}

"""Entry point that forwards a produced Result with capacity 4 and returns the Payload value as Int, or 0 on error."""
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
    let source = "import { open feature };\n\"\"\"Entry point that forwards a produced Result with capacity 4 and returns the Payload value as Int, or 0 on error.\"\"\"\nverb main() -> Int { erg result = forward[4](); return case dat result { Result.Ok(payload) => payload.value as Int, Result.Err(_) => 0, }; }\n";
    let (root, input, output) = project("nested-facade-aggregate-enum", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature facade");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature API."""
open api;
"#,
    )
    .expect("write feature facade");
    fs::write(
        root.join("src/feature/api.act"),
        r#""""A payload that carries one 32-bit value."""
open struct Payload {
    """The carried value."""
    erg value: u32,
}

"""The failure of the payload producers."""
open enum Failure {
    """The producer failed."""
    Failed,
}

"""Return a payload that carries 41 in the Ok case."""
open verb produce[N: Usize]() -> Result[Payload, Failure] { return Result[Payload, Failure].Ok(Payload { value: 41u32, }); }

"""Return the payload that produce returns, or its failure."""
open verb forward[N: Usize]() -> Result[Payload, Failure] { erg result = produce[N](); return case dat result { Result.Ok(payload) => Result[Payload, Failure].Ok(payload), Result.Err(error) => Result[Payload, Failure].Err(error), }; }
"#,
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

"""
contract:
purpose:
    Admit the storage by returning its u64 value as an Ok result.
inputs:
    storage: exclusive ins loan of Storage of capacity N holding a u64.
outputs:
    Result.Ok with the u64 value.
ownership:
    The storage is an exclusive ins loan and is only read.
"""
verb validate[N: Usize](ins storage: Storage[N]) -> Result[u64, Failure] {
    return Result[u64, Failure].Ok(storage.value);
}

"""
contract:
purpose:
    Validate the storage and wrap the admitted value into a TokenCode with a zero high mask.
inputs:
    storage: exclusive ins loan of Storage of capacity N.
outputs:
    Result.Ok with a TokenCode whose mask_low is the value, or the validation error.
ownership:
    The storage stays an exclusive ins loan and is passed on to validate.
errors:
    Any error from validate is passed through.
"""
verb encode[N: Usize](ins storage: Storage[N]) -> Result[TokenCode, Failure] {
    erg admission = validate(storage: ins storage);
    return case dat admission {
        Result.Err(error) => Result[TokenCode, Failure].Err(error),
        Result.Ok(value) => Result[TokenCode, Failure].Ok(TokenCode { mask_low: value, mask_high: 0u64, }),
    };
}

"""Entry point that encodes a storage holding 41 and returns the mask_low of the resulting TokenCode as Int, or 0 on error."""
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

"""
contract:
purpose:
    Wrap an Ok u64 admission of 41 into a TokenCode result, with each case arm ending in a block tail expression.
outputs:
    Result.Ok with a TokenCode whose mask_low is 41, or the admission error.
errors:
    The admission error is passed through; the local admission is always Ok here.
"""
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

"""Entry point that returns the mask_low of the TokenCode produced by encode as Int, or 0 on error."""
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
    let source = "import { open feature };\n\"\"\"Entry point that returns the result of run as the process exit status.\"\"\"\nverb main() -> Int { return run(); }\n";
    let (root, input, output) = project("nested-facade-result-aggregate-case", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(
        root.join("src/feature/feature.act"),
        r#""""Re-export the feature API."""
open api;
"#,
    )
    .expect("write feature facade");
    fs::write(root.join("src/feature/api.act"), r#""""Re-export the token types."""
open types;

"""Re-export the token encoding verbs."""
open encoding;

"""Encode a token and return its low mask, or 0 when encoding fails."""
open verb run() -> Int { erg result = encode(); return case dat result { Result.Ok(code) => code.mask_low as Int, Result.Err(_) => 0, }; }
"#)
        .expect("write feature api");
    fs::write(
        root.join("src/feature/types.act"),
        r#""""A token code of two 64-bit masks."""
open struct TokenCode {
    """The low mask."""
    erg mask_low: u64,
    """The high mask."""
    erg mask_high: u64,
}

"""The failure of token encoding."""
open enum Failure {
    """The token is invalid."""
    Invalid,
}
"#,
    )
    .expect("write feature types");
    fs::write(
        root.join("src/feature/encoding.act"),
        r#""""Return 41 in the Ok case."""
open verb validate() -> Result[u64, Failure] { return Result[u64, Failure].Ok(41u64); }

"""Encode the validated value as the low mask of a token code, or return the validation failure."""
open verb encode() -> Result[TokenCode, Failure] { erg admission = validate(); return case dat admission { Result.Err(error) => Result[TokenCode, Failure].Err(error), Result.Ok(value) => Result[TokenCode, Failure].Ok(TokenCode { mask_low: value, mask_high: 0u64, }), }; }
"#,
    )
    .expect("write feature encoding");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove nested facade result aggregate case project");
}

#[test]
fn result_case_binding_preserves_const_generic_array_payload() {
    let source = r#"
enum Failure {
    Invalid,
}

"""
contract:
purpose:
    Build a 20-byte array whose first byte is 41 and return it inside an Ok result.
outputs:
    Result.Ok owning the array, or Result.Err with Failure.
ownership:
    The array is created locally and moved to the caller.
"""
verb produce() -> Result[Array[u8, 20], Failure] {
    erg bytes: Array[u8,20] = Array[u8,20]();
    bytes[0u32] = 41u8;
    return Result[Array[u8, 20], Failure].Ok(bytes);
}

"""
contract:
purpose:
    Forward the array Result from produce, moving the Ok payload through the case.
outputs:
    Result.Ok owning the same array, or the same Failure.
ownership:
    The array moves from the produced result into the returned one.
errors:
    Failure values from produce are passed through.
"""
verb forward() -> Result[Array[u8, 20], Failure] {
    erg result = produce();
    return case dat result {
        Result.Ok(bytes) => Result[Array[u8, 20], Failure].Ok(bytes),
        Result.Err(error) => Result[Array[u8, 20], Failure].Err(error),
    };
}

"""Entry point that returns the first byte of the forwarded array as Int, or 0 when forward fails."""
verb main() -> Int {
    erg result = forward();
    return case dat result {
        Result.Ok(bytes) => bytes[0u32] as Int,
        Result.Err(_) => 0,
    };
}
"#;
    let (root, input, output) = project("result-case-array-payload", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove result case array payload project");
}

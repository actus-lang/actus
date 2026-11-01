use super::*;
use std::fs;
use std::process::Command;

fn emit_runtime_status_bridge(
    external_name: &str,
    parameter_type: BackendValueType,
    return_type: BackendValueType,
) -> String {
    let external_symbol =
        SymbolIdentity::new("runtime", SymbolKind::External, external_name).expect("valid symbol");
    let external = BackendExternal {
        symbol: external_symbol.clone(),
        parameters: vec![BackendParameter {
            name: "input".to_owned(),
            role: Role::Abs,
            ty: parameter_type,
            passing: BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: return_type,
            passing: BackendReturnMode::Direct,
        },
        runtime_pointer_return: false,
    };
    let function = BackendFunction {
        symbol: SymbolIdentity::new("runtime_test", SymbolKind::Verb, "call_status")
            .expect("valid caller symbol"),
        hosted_entry_return: false,
        parameters: vec![BackendParameter {
            name: "input".to_owned(),
            role: Role::Erg,
            ty: parameter_type,
            passing: BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: return_type,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: external_symbol,
                    arguments: vec![BackendValueId(0)],
                    result: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    emit_module(&BackendModule { externals: vec![external], functions: vec![function] })
        .expect("runtime bridge ABI should lower through LLVM")
}

fn emit_source_runtime_bridge(source: &str) -> String {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "runtime bridge fixture should lex: {errors:?}");
    let program = parse(tokens).expect("runtime bridge fixture should parse");
    let module = lower_scalar_program("runtime_test", &program, Some("call"))
        .expect("runtime bridge fixture should lower through the shared contract");
    emit_module(&module).expect("runtime bridge fixture should lower through LLVM")
}

fn run_llvm_region_fixture(name: &str, source: &str) -> i32 {
    let root =
        std::env::temp_dir().join(format!("actus-llvm-region-{name}-{}", std::process::id()));
    let source_root = root.join("src");
    let input = source_root.join("main.act");
    let output = root.join("region-fixture");
    fs::create_dir_all(&source_root).expect("Region fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm_region_bridge_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nbackend = \"llvm\"\nverify_no_float_ir = true\n",
    )
    .expect("Region fixture manifest should be written");
    fs::write(&input, source).expect("Region fixture source should be written");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            input.to_str().expect("Region fixture input should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("Region fixture output should be valid UTF-8"),
        ])
        .output()
        .expect("LLVM Region fixture build should start");
    assert!(
        build.status.success(),
        "LLVM Region fixture build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let status = Command::new(&output).status().expect("LLVM Region fixture should run");
    let code = status.code().expect("Region fixture should return an exit code");
    let _ = fs::remove_dir_all(root);
    code
}

#[test]
fn llvm_preserves_i32_runtime_status_through_a_bridge_call() {
    let ir = emit_runtime_status_bridge(
        "file_close",
        BackendValueType::Integer { signed: true, width: 64 },
        BackendValueType::Integer { signed: true, width: 32 },
    );
    assert!(ir.contains("declare i32 @runtime__extern_file_5fclose(i64)"));
    assert!(ir.contains("call i32 @runtime__extern_file_5fclose(i64 %0)"));
    assert!(ir.contains("ret i32 %call1"));
}

#[test]
fn llvm_preserves_i64_runtime_handle_status_through_a_bridge_call() {
    let ir = emit_runtime_status_bridge(
        "open_handle",
        BackendValueType::Pointer,
        BackendValueType::Integer { signed: true, width: 64 },
    );
    assert!(ir.contains("declare i64 @runtime__extern_open_5fhandle(ptr)"));
    assert!(ir.contains("call i64 @runtime__extern_open_5fhandle(ptr %0)"));
    assert!(ir.contains("ret i64 %call1"));
}

#[test]
fn llvm_preserves_pointer_runtime_result_without_status_coercion() {
    let ir = emit_runtime_status_bridge(
        "allocate_handle",
        BackendValueType::Integer { signed: false, width: 64 },
        BackendValueType::Pointer,
    );
    assert!(ir.contains("declare ptr @runtime__extern_allocate_5fhandle(i64)"));
    assert!(ir.contains("call ptr @runtime__extern_allocate_5fhandle(i64 %0)"));
    assert!(ir.contains("ret ptr %call1"));
}

#[test]
fn llvm_lowers_the_filesystem_close_bridge_with_its_status_contract() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_close_buffer(abs handle: Int) -> Int; verb call(erg handle: Int) -> Int { return actus_file_close_buffer(handle: abs handle); }",
    );
    assert!(ir.contains("declare i32 @actus_file_close_buffer(i32)"));
    assert!(ir.contains("call i32 @actus_file_close_buffer(i32 %0)"));
    assert!(ir.contains("ret i32 %call1"));
}

#[test]
fn llvm_lowers_the_monotonic_clock_bridge_without_hidden_parameters() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_monotonic_nanos() -> u64; verb call() -> u64 { return actus_monotonic_nanos(); }",
    );
    assert!(ir.contains("declare i64 @actus_monotonic_nanos()"));
    assert!(ir.contains("call i64 @actus_monotonic_nanos()"));
    assert!(ir.contains("ret i64 %call0"));
}

#[test]
fn llvm_lowers_the_process_count_bridge_as_a_scalar_status() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_process_arg_count() -> Int; verb call() -> Int { return actus_process_arg_count(); }",
    );
    assert!(ir.contains("declare i32 @actus_process_arg_count()"));
    assert!(ir.contains("call i32 @actus_process_arg_count()"));
    assert!(ir.contains("ret i32 %call0"));
}

#[test]
fn llvm_lowers_the_stdin_eof_bridge_as_a_signed_status() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_read_byte() -> Int; verb call() -> Int { return actus_read_byte(); }",
    );
    assert!(ir.contains("declare i32 @actus_read_byte()"));
    assert!(ir.contains("call i32 @actus_read_byte()"));
}

#[test]
fn llvm_lowers_the_stdout_flush_bridge_as_a_signed_status() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_flush_stdout() -> Int; verb call() -> Int { return actus_flush_stdout(); }",
    );
    assert!(ir.contains("declare i32 @actus_flush_stdout()"));
    assert!(ir.contains("call i32 @actus_flush_stdout()"));
}

#[test]
fn llvm_lowers_the_stdout_buffer_bridge_through_a_borrowed_pointer() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_write_buffer_stdout(abs buffer: Buffer) -> Int; verb call(abs buffer: Buffer) -> Int { return actus_write_buffer_stdout(buffer: abs buffer); }",
    );
    assert!(ir.contains("declare i32 @actus_write_buffer_stdout(ptr align 8)"), "{ir}");
    assert!(ir.contains("call i32 @actus_write_buffer_stdout(ptr %0)"), "{ir}");
}

#[test]
fn llvm_executes_the_region_open_and_close_bridge_contract() {
    let source = "import { open std::region }; meta effect(\"allocation\") verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg closed = region_close(region: ins region); return case dat closed { Result.Err(_) => 2, Result.Ok(_) => 0, }; }, }; }";
    assert_eq!(run_llvm_region_fixture("open-close", source), 0);
}

#[test]
fn llvm_preserves_the_region_bounds_failure_contract() {
    let source = "import { open std::region }; meta effect(\"allocation\") verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg destination: Buffer = Buffer[4]; erg index: u64 = 1u64; erg read = region_read(region: abs region, index: erg index, destination: ins destination); return case dat read { Result.Err(_) => 0, Result.Ok(_) => 2, }; }, }; }";
    assert_eq!(run_llvm_region_fixture("bounds-failure", source), 0);
}

#[test]
fn llvm_lowers_the_buffer_length_bridge_through_a_borrowed_pointer() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_buffer_length(abs buffer: Buffer) -> Int; verb call(abs buffer: Buffer) -> Int { return actus_buffer_length(buffer: abs buffer); }",
    );
    assert!(ir.contains("declare i32 @actus_buffer_length(ptr align 8)"));
    assert!(ir.contains("call i32 @actus_buffer_length(ptr %0)"));
}

#[test]
fn llvm_lowers_the_buffer_byte_access_bridge_with_a_signed_index() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_buffer_byte_at(abs buffer: Buffer, erg index: Int) -> Int; verb call(abs buffer: Buffer, erg index: Int) -> Int { return actus_buffer_byte_at(buffer: abs buffer, index: erg index); }",
    );
    assert!(ir.contains("declare i32 @actus_buffer_byte_at(ptr align 8, i32)"));
    assert!(ir.contains("call i32 @actus_buffer_byte_at(ptr %0, i32 %1)"));
}

#[test]
fn llvm_lowers_the_buffer_utf8_validation_bridge_as_a_signed_status() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_buffer_validate_utf8(abs buffer: Buffer) -> Int; verb call(abs buffer: Buffer) -> Int { return actus_buffer_validate_utf8(buffer: abs buffer); }",
    );
    assert!(ir.contains("declare i32 @actus_buffer_validate_utf8(ptr align 8)"));
    assert!(ir.contains("call i32 @actus_buffer_validate_utf8(ptr %0)"));
}

#[test]
fn llvm_lowers_the_filesystem_remove_path_bridge_with_a_borrowed_pointer() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_remove_path(abs path: Buffer) -> Int; verb call(abs path: Buffer) -> Int { return actus_file_remove_path(path: abs path); }",
    );
    assert!(ir.contains("declare i32 @actus_file_remove_path(ptr align 8)"));
    assert!(ir.contains("call i32 @actus_file_remove_path(ptr %0)"));
}

#[test]
fn llvm_lowers_the_filesystem_parent_sync_bridge_with_a_status_return() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_sync_parent_path(abs path: Buffer) -> Int; verb call(abs path: Buffer) -> Int { return actus_file_sync_parent_path(path: abs path); }",
    );
    assert!(ir.contains("declare i32 @actus_file_sync_parent_path(ptr align 8)"));
    assert!(ir.contains("call i32 @actus_file_sync_parent_path(ptr %0)"));
}

#[test]
fn llvm_lowers_the_directory_close_bridge_with_a_scalar_handle() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_directory_close(abs handle: Int) -> Int; verb call(erg handle: Int) -> Int { return actus_directory_close(handle: abs handle); }",
    );
    assert!(ir.contains("declare i32 @actus_directory_close(i32)"));
    assert!(ir.contains("call i32 @actus_directory_close(i32 %0)"));
}

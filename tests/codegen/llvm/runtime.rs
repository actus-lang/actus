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

fn run_llvm_executable_fixture(name: &str, source: &str) -> i32 {
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
fn llvm_preserves_the_external_c_scalar_abi_matrix() {
    for (actus_type, llvm_type) in [
        ("Bool", "i1"),
        ("u8", "i8"),
        ("u16", "i16"),
        ("Int", "i32"),
        ("u64", "i64"),
        ("Buffer", "ptr"),
    ] {
        let source = format!(
            "unsafe extern \"C\" verb bridge() -> {actus_type}; verb call() -> {actus_type} {{ return bridge(); }}"
        );
        let ir = emit_source_runtime_bridge(&source);
        assert!(ir.contains(&format!("declare {llvm_type} @bridge()")), "{ir}");
        assert!(ir.contains(&format!("call {llvm_type} @bridge()")), "{ir}");
    }
}

#[test]
fn llvm_lowers_the_filesystem_read_bridge_with_a_mutable_buffer() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_read_buffer(abs handle: Int, ins buffer: Buffer) -> Int; verb call(erg handle: Int, ins buffer: Buffer) -> Int { return actus_file_read_buffer(handle: abs handle, buffer: ins buffer); }",
    );
    assert!(ir.contains("declare i32 @actus_file_read_buffer(i32, ptr align 8)"), "{ir}");
    assert!(ir.contains("call i32 @actus_file_read_buffer(i32 %0, ptr %1)"), "{ir}");
}

#[test]
fn llvm_lowers_the_filesystem_write_bridge_with_a_borrowed_buffer() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_write_buffer(abs handle: Int, abs buffer: Buffer) -> Int; verb call(erg handle: Int, abs buffer: Buffer) -> Int { return actus_file_write_buffer(handle: abs handle, buffer: abs buffer); }",
    );
    assert!(ir.contains("declare i32 @actus_file_write_buffer(i32, ptr align 8)"), "{ir}");
    assert!(ir.contains("call i32 @actus_file_write_buffer(i32 %0, ptr %1)"), "{ir}");
}

#[test]
fn llvm_lowers_the_filesystem_seek_bridge_with_three_signed_scalars() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_seek_buffer(abs handle: Int, abs offset: Int, abs whence: Int) -> Int; verb call(erg handle: Int, erg offset: Int, erg whence: Int) -> Int { return actus_file_seek_buffer(handle: abs handle, offset: abs offset, whence: abs whence); }",
    );
    assert!(ir.contains("declare i32 @actus_file_seek_buffer(i32, i32, i32)"), "{ir}");
    assert!(ir.contains("call i32 @actus_file_seek_buffer(i32 %0, i32 %1, i32 %2)"), "{ir}");
}

#[test]
fn llvm_lowers_the_filesystem_sync_bridges_as_scalar_statuses() {
    for symbol in ["actus_file_sync_data_buffer", "actus_file_sync_all_buffer"] {
        let source = format!(
            "unsafe extern \"C\" verb {symbol}(abs handle: Int) -> Int; verb call(erg handle: Int) -> Int {{ return {symbol}(handle: abs handle); }}"
        );
        let ir = emit_source_runtime_bridge(&source);
        assert!(ir.contains(&format!("declare i32 @{symbol}(i32)")), "{ir}");
        assert!(ir.contains(&format!("call i32 @{symbol}(i32 %0)")), "{ir}");
    }
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
    assert_eq!(run_llvm_executable_fixture("open-close", source), 0);
}

#[test]
fn llvm_preserves_the_region_bounds_failure_contract() {
    let source = "import { open std::region }; meta effect(\"allocation\") verb main() -> Int { erg backing: Buffer = Buffer[4]; erg logical_length: u64 = 1u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[u32](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { erg destination: Buffer = Buffer[4]; erg index: u64 = 1u64; erg read = region_read(region: abs region, index: erg index, destination: ins destination); return case dat read { Result.Err(_) => 0, Result.Ok(_) => 2, }; }, }; }";
    assert_eq!(run_llvm_executable_fixture("bounds-failure", source), 0);
}

#[test]
fn llvm_preserves_representative_runtime_failure_statuses() {
    let source = "unsafe extern \"C\" verb actus_file_close_buffer(abs handle: Int) -> Int; unsafe extern \"C\" verb actus_process_arg(ins buffer: Buffer, erg index: Int) -> Int; unsafe extern \"C\" verb actus_path_validate_storage(abs storage: Buffer, erg unit_width: Int) -> Int; unsafe extern \"C\" verb actus_buffer_byte_at(abs buffer: Buffer, erg index: Int) -> Int; unsafe extern \"C\" verb actus_buffer_validate_utf8(abs buffer: Buffer) -> Int; verb main() -> Int { erg handle: Int = -1; erg index: Int = -1; erg unit_width: Int = 1; erg byte_index: Int = 0; erg empty: Buffer = Buffer[0]; erg invalid: Buffer = Buffer[0]; append(invalid, 255u8); erg valid: Buffer = Buffer[0]; append(valid, 65u8); erg close_status = actus_file_close_buffer(handle: abs handle); erg process_status = actus_process_arg(buffer: ins empty, index: erg index); erg path_status = actus_path_validate_storage(storage: abs empty, unit_width: erg unit_width); erg empty_byte_status = actus_buffer_byte_at(buffer: abs empty, index: erg unit_width); erg invalid_utf8_status = actus_buffer_validate_utf8(buffer: abs invalid); erg invalid_byte = actus_buffer_byte_at(buffer: abs invalid, index: erg byte_index); erg valid_utf8_status = actus_buffer_validate_utf8(buffer: abs valid); erg valid_byte = actus_buffer_byte_at(buffer: abs valid, index: erg byte_index); drop(empty); drop(invalid); drop(valid); if close_status != -1 { return 1; } if process_status != -3 { return 2; } if path_status != -4 { return 3; } if empty_byte_status != -2 { return 4; } if invalid_utf8_status != -1 { return 5; } if invalid_byte != 255 { return 6; } if valid_utf8_status != 0 { return 7; } if valid_byte != 65 { return 8; } return 0; }";
    assert_eq!(run_llvm_executable_fixture("runtime-failure-statuses", source), 0);
}

#[test]
fn llvm_executes_the_deterministic_runtime_status_matrix() {
    let source = "unsafe extern \"C\" verb actus_process_arg_count() -> Int; unsafe extern \"C\" verb actus_process_arg(ins buffer: Buffer, erg index: Int) -> Int; unsafe extern \"C\" verb actus_string_length(abs text: String) -> Int; unsafe extern \"C\" verb actus_string_byte_at(abs text: String, erg index: Int) -> Int; unsafe extern \"C\" verb actus_buffer_capacity(abs buffer: Buffer) -> Int; unsafe extern \"C\" verb actus_buffer_clear(ins buffer: Buffer) -> Int; unsafe extern \"C\" verb actus_cursor_seek(ins buffer: Buffer, erg position: Int) -> Int; unsafe extern \"C\" verb actus_sleep_context_enter(erg flags: u32) -> Int; unsafe extern \"C\" verb actus_sleep_context_exit(erg flags: u32) -> Int; unsafe extern \"C\" verb actus_sleep_nanos(erg nanos: u64) -> Int; verb main() -> Int { erg empty: Buffer = Buffer[0]; erg text: String = \"Actus\"; erg invalid_index: Int = 999999; erg first_index: Int = 0; erg negative_index: Int = -1; erg end_index: Int = 5; erg negative_position: Int = -1; erg invalid_flags: u32 = 0u32; erg context_flags: u32 = 1u32; erg no_sleep: u64 = 0u64; erg process_count = actus_process_arg_count(); erg process_invalid = actus_process_arg(buffer: ins empty, index: erg invalid_index); erg string_length = actus_string_length(text: abs text); erg string_byte = actus_string_byte_at(text: abs text, index: erg first_index); erg string_negative = actus_string_byte_at(text: abs text, index: erg negative_index); erg string_oob = actus_string_byte_at(text: abs text, index: erg end_index); erg capacity = actus_buffer_capacity(buffer: abs empty); erg clear = actus_buffer_clear(buffer: ins empty); erg cursor_negative = actus_cursor_seek(buffer: ins empty, position: erg negative_position); erg context_invalid = actus_sleep_context_enter(flags: erg invalid_flags); erg context_enter = actus_sleep_context_enter(flags: erg context_flags); erg blocked_sleep = actus_sleep_nanos(nanos: erg no_sleep); erg context_exit = actus_sleep_context_exit(flags: erg context_flags); erg context_mismatch = actus_sleep_context_exit(flags: erg context_flags); drop(empty); if process_count < 1 { return 1; } if process_invalid != -3 { return 2; } if string_length != 5 { return 3; } if string_byte != 65 { return 4; } if string_negative != -3 { return 5; } if string_oob != -3 { return 6; } if capacity != 0 { return 7; } if clear != 0 { return 8; } if cursor_negative != -1 { return 9; } if context_invalid != -1 { return 10; } if context_enter != 0 { return 11; } if blocked_sleep != -1 { return 12; } if context_exit != 0 { return 13; } if context_mismatch != -1 { return 14; } drop(text); return 0; }";
    assert_eq!(run_llvm_executable_fixture("deterministic-status-matrix", source), 0);
}

#[test]
fn llvm_executes_checked_representation_layouts() {
    let source = "meta repr(C, packed) struct Wire { prefix: u8, value: u32, } meta repr(transparent) struct UserId { value: u64, } meta repr(C, align(8u32)) struct Aligned { value: u8, } meta repr(C) enum Message { Byte { value: u8, }, Pair { code: u16, value: u32, }, Empty, } verb main() -> Int { erg wire: Wire = Wire { prefix: 1u8, value: 40u32, }; erg id: UserId = UserId { value: 2u64, }; erg aligned: Aligned = Aligned { value: 0u8, }; erg message = Message.Pair(code: 0u16, value: 0u32); wire.value = 41u32; id.value = 1u64; aligned.value = 0u8; return case dat message { Message.Pair { code: code_value, value: value_value } => wire.value as Int + id.value as Int + aligned.value as Int + code_value as Int + value_value as Int, _ => 1, }; }";
    assert_eq!(run_llvm_executable_fixture("checked-representations", source), 42);
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
fn llvm_lowers_buffer_allocation_with_a_pointer_result() {
    let ir = emit_source_runtime_bridge("verb call() -> Buffer { return Buffer[4]; }");
    assert!(ir.contains("declare ptr @actus_buffer_allocate(i64)"), "{ir}");
    assert!(ir.contains("call ptr @actus_buffer_allocate(i64 4)"), "{ir}");
}

#[test]
fn llvm_lowers_buffer_append_with_an_eight_bit_byte() {
    let ir = emit_source_runtime_bridge(
        "verb call() -> Int { erg buffer: Buffer = Buffer[0]; append(handle: buffer, byte: 7u8); return 0; }",
    );
    assert!(ir.contains("declare i8 @actus_buffer_append(ptr, i8)"), "{ir}");
    assert!(ir.contains("call i8 @actus_buffer_append(ptr %"), "{ir}");
}

#[test]
fn llvm_lowers_buffer_byte_set_with_checked_index_and_byte_types() {
    let ir = emit_source_runtime_bridge(
        "verb call(ins buffer: Buffer, erg index: Int, erg byte: u8) -> Int { buffer[index] = byte; return 0; }",
    );
    assert!(ir.contains("declare i32 @actus_buffer_byte_set(ptr, i32, i8)"), "{ir}");
    assert!(ir.contains("call i32 @actus_buffer_byte_set(ptr %0, i32 %1, i8 %2)"), "{ir}");
}

#[test]
fn llvm_lowers_buffer_move_as_a_void_ownership_bridge() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_buffer_move(abs destination: Buffer, abs source: Buffer); verb call(abs destination: Buffer, abs source: Buffer) -> Int { actus_buffer_move(destination: abs destination, source: abs source); return 0; }",
    );
    assert!(ir.contains("declare void @actus_buffer_move(ptr align 8, ptr align 8)"), "{ir}");
    assert!(ir.contains("call void @actus_buffer_move(ptr %0, ptr %1)"), "{ir}");
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

#[test]
fn llvm_lowers_the_atomic_replace_bridge_with_two_borrowed_paths() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_replace_atomic_path(abs from: Buffer, abs to: Buffer) -> Int; verb call(abs from: Buffer, abs to: Buffer) -> Int { return actus_file_replace_atomic_path(from: abs from, to: abs to); }",
    );
    assert!(ir.contains("declare i32 @actus_file_replace_atomic_path(ptr align 8, ptr align 8)"));
    assert!(ir.contains("call i32 @actus_file_replace_atomic_path(ptr %0, ptr %1)"));
}

#[test]
fn llvm_lowers_the_shared_lock_bridge_with_a_scalar_handle() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_lock_shared(abs handle: Int) -> Int; verb call(erg handle: Int) -> Int { return actus_file_lock_shared(handle: abs handle); }",
    );
    assert!(ir.contains("declare i32 @actus_file_lock_shared(i32)"));
    assert!(ir.contains("call i32 @actus_file_lock_shared(i32 %0)"));
}

#[test]
fn llvm_lowers_the_try_lock_busy_status_bridge_without_coercion() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_try_lock(abs handle: Int) -> Int; verb call(erg handle: Int) -> Int { return actus_file_try_lock(handle: abs handle); }",
    );
    assert!(ir.contains("declare i32 @actus_file_try_lock(i32)"));
    assert!(ir.contains("call i32 @actus_file_try_lock(i32 %0)"));
}

#[test]
fn llvm_lowers_the_unlock_bridge_with_a_scalar_status_return() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_unlock(abs handle: Int) -> Int; verb call(erg handle: Int) -> Int { return actus_file_unlock(handle: abs handle); }",
    );
    assert!(ir.contains("declare i32 @actus_file_unlock(i32)"));
    assert!(ir.contains("call i32 @actus_file_unlock(i32 %0)"));
}

#[test]
fn llvm_lowers_the_open_options_bridge_with_all_flag_arguments() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_file_open_options_path(abs path: Buffer, abs read: Int, abs write: Int, abs append: Int, abs truncate: Int, abs create: Int, abs create_new: Int, abs close_on_exec: Int) -> Int; verb call(abs path: Buffer, erg flag: Int) -> Int { return actus_file_open_options_path(path: abs path, read: abs flag, write: abs flag, append: abs flag, truncate: abs flag, create: abs flag, create_new: abs flag, close_on_exec: abs flag); }",
    );
    assert!(ir.contains(
        "declare i32 @actus_file_open_options_path(ptr align 8, i32, i32, i32, i32, i32, i32, i32)"
    ));
    assert!(ir.contains("call i32 @actus_file_open_options_path(ptr %0, i32 %1, i32 %1, i32 %1, i32 %1, i32 %1, i32 %1, i32 %1)"));
}

#[test]
fn llvm_lowers_the_ordered_directory_open_bridge_with_a_order_argument() {
    let ir = emit_source_runtime_bridge(
        "unsafe extern \"C\" verb actus_directory_open_ordered(abs path: Buffer, erg order: Int) -> Int; verb call(abs path: Buffer, erg order: Int) -> Int { return actus_directory_open_ordered(path: abs path, order: erg order); }",
    );
    assert!(ir.contains("declare i32 @actus_directory_open_ordered(ptr align 8, i32)"));
    assert!(ir.contains("call i32 @actus_directory_open_ordered(ptr %0, i32 %1)"));
}

#[test]
fn llvm_lowers_repr_c_external_aggregate_parameters_without_erasing_the_layout() {
    let ir = emit_source_runtime_bridge(
        "meta repr(C) struct Point { x: i32, y: i32, } unsafe extern \"C\" verb inspect(erg point: Point) -> Int; verb call(erg point: Point) -> Int { return inspect(point: erg point); }",
    );
    assert!(ir.contains("declare i32 @inspect(ptr align 4)"), "{ir}");
    assert!(ir.contains("call i32 @inspect(ptr %0)"), "{ir}");
}

#[test]
fn llvm_lowers_repr_c_external_aggregate_returns_through_the_declared_return_abi() {
    let ir = emit_source_runtime_bridge(
        "meta repr(C) struct Point { x: i32, y: i32, } unsafe extern \"C\" verb make() -> Point; verb call() -> Point { return make(); }",
    );
    assert!(ir.contains("declare void @make(ptr align 4)"), "{ir}");
    assert!(ir.contains("call void @make(ptr %slot0)"), "{ir}");
}

#![cfg(feature = "llvm")]

use actus::ast::Role;
#[cfg(feature = "cranelift")]
use actus::codegen::cranelift::emit_contract_module_object;
use actus::codegen::link_objects;
use actus::codegen::llvm::{emit_module, emit_object};
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendEnumLayout, BackendExternal, BackendFailure,
    BackendFunction, BackendInstruction, BackendModule, BackendParameter, BackendReturn,
    BackendReturnMode, BackendValueId, BackendValueType, LoweredCleanupInstruction, SymbolIdentity,
    SymbolKind,
};
use actus::configuration::CompilerConfiguration;
use object::{Object, ObjectSymbol};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[test]
fn emits_a_valid_unit_function_as_llvm_ir() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
            cleanup: Vec::new(),
        }],
    };

    let ir = emit_module(&module).expect("unit function should lower to LLVM IR");
    assert!(ir.contains("define void @test__verb_main()"));
    assert!(ir.contains("ret void"));
}

#[test]
fn emits_scalar_parameter_return_and_direct_call() {
    let integer = BackendValueType::Integer { signed: false, width: 32 };
    let callee_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "add_one").expect("valid callee symbol");
    let caller_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid caller symbol");
    let parameter = BackendParameter {
        name: "input".to_owned(),
        role: Role::Erg,
        ty: integer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let callee = BackendFunction {
        symbol: callee_symbol.clone(),
        parameters: vec![parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let caller = BackendFunction {
        symbol: caller_symbol,
        parameters: vec![parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: callee_symbol,
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
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![callee, caller] })
        .expect("scalar call should lower to LLVM IR");
    assert!(ir.contains("define i32 @test__verb_add_5fone"));
    assert!(ir.contains("define i32 @test__verb_main"));
    assert!(ir.contains("call i32 @test__verb_add_5fone"));
}

#[test]
fn declares_and_calls_a_neutral_external_runtime_symbol() {
    let external_symbol =
        SymbolIdentity::new("runtime", SymbolKind::External, "write_byte").expect("valid symbol");
    let external = BackendExternal {
        symbol: external_symbol.clone(),
        parameters: vec![BackendParameter {
            name: "byte".to_owned(),
            role: Role::Abs,
            ty: BackendValueType::Integer { signed: false, width: 32 },
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
    };
    let function = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "runtime_call")
            .expect("valid symbol"),
        parameters: vec![BackendParameter {
            name: "byte".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Integer { signed: false, width: 32 },
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: external_symbol,
                    arguments: vec![BackendValueId(0)],
                    result: None,
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: None,
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: vec![external], functions: vec![function] })
        .expect("neutral external should be declared and called");
    assert!(ir.contains("declare void @runtime__extern_write_5fbyte(i32)"));
    assert!(ir.contains("call void @runtime__extern_write_5fbyte(i32 %0)"));
}

#[test]
fn emits_scalar_branch_jump_and_trap_terminators() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "control").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "condition".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Bool,
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![
            BackendBlock {
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Branch {
                    condition: BackendValueId(0),
                    then_block: BackendBlockId(1),
                    else_block: BackendBlockId(2),
                    span: actus::lexer::SourceSpan::new(0, 1),
                }],
            },
            BackendBlock {
                id: BackendBlockId(1),
                instructions: vec![BackendInstruction::Jump {
                    target: BackendBlockId(3),
                    span: actus::lexer::SourceSpan::new(1, 2),
                }],
            },
            BackendBlock {
                id: BackendBlockId(2),
                instructions: vec![BackendInstruction::Trap {
                    reason: "false branch".to_owned(),
                    span: actus::lexer::SourceSpan::new(2, 3),
                }],
            },
            BackendBlock {
                id: BackendBlockId(3),
                instructions: vec![BackendInstruction::Return {
                    value: None,
                    span: actus::lexer::SourceSpan::new(3, 4),
                }],
            },
        ],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("scalar control flow should lower to LLVM IR");
    assert!(ir.contains("br i1 %0, label %bb1, label %bb2"));
    assert!(ir.contains("br label %bb3"));
    assert!(ir.contains("unreachable"));
}

#[test]
fn lowers_end_borrow_cleanup_as_a_compile_time_boundary() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "cleanup").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Cleanup {
                    action: LoweredCleanupInstruction::EndBorrow { borrow_id: 3 },
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: None,
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("end-borrow cleanup should lower without a runtime call");
    assert!(ir.contains("define void @test__verb_cleanup()"));
    assert!(ir.contains("ret void"));
}

#[test]
fn lowers_pointer_memory_copy_with_declared_extent_and_alignment() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "copy").expect("valid symbol");
    let pointer = BackendParameter {
        name: "pointer".to_owned(),
        role: actus::ast::Role::Abs,
        ty: BackendValueType::Pointer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let function = BackendFunction {
        symbol,
        parameters: vec![
            pointer.clone(),
            BackendParameter { name: "target".to_owned(), ..pointer },
        ],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::MemoryCopy {
                    destination: BackendValueId(1),
                    source: BackendValueId(0),
                    size: 16,
                    alignment: 8,
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: None,
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("pointer memory copy should lower to LLVM memmove");
    assert!(ir.contains("llvm.memmove"));
    assert!(ir.contains("i64 16"));
}

#[test]
fn lowers_regular_enum_failure_with_discriminant_and_integer_payload() {
    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let enum_layout = BackendEnumLayout {
        layout,
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset: 8,
        payload: Some(actus::codegen::BackendLayout { size: 4, alignment: 4 }),
        niche_pointer: false,
    };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "failure").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "error".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Integer { signed: true, width: 32 },
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::Aggregate { layout },
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Result")
                        .expect("valid result symbol"),
                    discriminant: 1,
                    payload: Some(BackendValueId(0)),
                    layout: enum_layout,
                },
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("regular enum failure should lower to an aggregate return");
    assert!(ir.contains("store i32 1"));
    assert!(ir.contains("store i32 %0"));
    assert!(ir.contains("ret [2 x i64]"));
}

#[test]
fn rejects_niche_enum_failure_until_pointer_niche_lowering_exists() {
    let layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "niche_failure").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::Aggregate { layout },
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Option")
                        .expect("valid option symbol"),
                    discriminant: 1,
                    payload: None,
                    layout: BackendEnumLayout {
                        layout,
                        discriminant_offset: 0,
                        discriminant_size: 4,
                        payload_offset: 0,
                        payload: None,
                        niche_pointer: true,
                    },
                },
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let error = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect_err("niche enum lowering is intentionally not approximated");
    assert!(error.0.contains("niche-pointer failure lowering is not implemented"));
}

#[test]
fn rejects_floating_point_types_before_llvm_lowering() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "float_value").expect("symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Float { width: 32 },
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
            cleanup: Vec::new(),
        }],
    };
    let error = emit_module(&module).expect_err("LLVM integer-only lowering must reject floats");
    assert!(error.to_string().contains("integer-only emission"));
}

#[test]
fn emits_fixed_layout_aggregate_parameter_and_return() {
    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let aggregate = BackendValueType::Aggregate { layout };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "aggregate").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: aggregate,
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("fixed-layout aggregate should lower to LLVM IR");
    assert!(ir.contains("define [2 x i64] @test__verb_aggregate([2 x i64] %0)"));
    assert!(ir.contains("ret [2 x i64] %0"));
}

#[test]
fn emits_non_direct_aggregate_parameter_as_aligned_pointer() {
    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "loan").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "storage".to_owned(),
            role: Role::Ins,
            ty: BackendValueType::Aggregate { layout },
            passing: actus::codegen::BackendPassingMode::ExclusiveLoan { layout },
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("non-direct aggregate parameter should lower to a pointer");
    assert!(ir.contains("define void @test__verb_loan(ptr align 8 %0)"));
}

#[test]
fn lowers_aggregate_return_slot_between_caller_and_callee() {
    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let aggregate = BackendValueType::Aggregate { layout };
    let callee_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "make").expect("valid callee symbol");
    let caller_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "use").expect("valid caller symbol");
    let parameter = BackendParameter {
        name: "value".to_owned(),
        role: Role::Erg,
        ty: aggregate,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let callee = BackendFunction {
        symbol: callee_symbol.clone(),
        parameters: vec![parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::ReturnSlot { layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let caller = BackendFunction {
        symbol: caller_symbol,
        parameters: vec![parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: callee_symbol,
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
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![callee, caller] })
        .expect("aggregate return slot should lower to LLVM IR");
    assert!(ir.contains("define void @test__verb_make(ptr align 8"));
    assert!(ir.contains("call void @test__verb_make(ptr"));
    assert!(ir.contains("store "));
    assert!(ir.contains("load [2 x i64]"));
}

#[test]
fn emits_a_host_object_with_a_stable_public_symbol() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
            cleanup: Vec::new(),
        }],
    };
    let output = std::env::temp_dir().join(format!("actus-llvm-object-{}.o", std::process::id()));
    let second_output =
        std::env::temp_dir().join(format!("actus-llvm-object-{}-second.o", std::process::id()));
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    emit_object(&module, &target, &output).expect("host object should be emitted");
    emit_object(&module, &target, &second_output).expect("second host object should be emitted");
    let bytes = std::fs::read(&output).expect("object should be readable");
    let second_bytes = std::fs::read(&second_output).expect("second object should be readable");
    assert_eq!(bytes, second_bytes, "repeated object emission must be deterministic");
    let object = object::File::parse(bytes.as_slice()).expect("object should parse");
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("test__verb_main")));
    std::fs::remove_file(output).expect("test object should be removed");
    std::fs::remove_file(second_output).expect("second test object should be removed");
}

#[cfg(target_os = "linux")]
#[test]
fn links_and_runs_a_host_object_through_the_shared_linker() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-llvm-link-{}", std::process::id()));
    fs::create_dir_all(&root).expect("link fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"llvm_link_fixture\"\nversion = \"0.1.0\"\n")
        .expect("link fixture manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern void test__verb_main(void); int main(void) { test__verb_main(); return 0; }\n",
    )
    .expect("entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
            cleanup: Vec::new(),
        }],
    };
    let llvm_object = root.join("module.o");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    emit_object(&module, &target, &llvm_object).expect("LLVM object should be emitted");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("link fixture configuration should load");
    let executable = root.join("linked");
    let objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    link_objects(&objects, &executable, &configuration).expect("host objects should link");
    let status = Command::new(&executable).status().expect("linked executable should run");
    assert_eq!(status.code(), Some(0));
    remove_fixture(&root, [&manifest, &shim_source, &shim_object, &llvm_object, &executable]);
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_the_same_scalar_contract_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-backend-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"backend_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("parity fixture manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern int test__verb_entry(int); extern void test__verb_control(unsigned char); int main(void) { if (test__verb_entry(42) != 42) return 1; test__verb_control(1); return 0; }\n",
    )
    .expect("parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let identity_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "identity").expect("identity symbol");
    let entry_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "entry").expect("entry symbol");
    let control_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "control").expect("control symbol");
    let parameter = BackendParameter {
        name: "input".to_owned(),
        role: Role::Erg,
        ty: integer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let identity = BackendFunction {
        symbol: identity_symbol.clone(),
        parameters: vec![parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let entry = BackendFunction {
        symbol: entry_symbol,
        parameters: vec![parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: identity_symbol,
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
    let control = BackendFunction {
        symbol: control_symbol,
        parameters: vec![BackendParameter {
            name: "condition".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Bool,
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![
            BackendBlock {
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Branch {
                    condition: BackendValueId(0),
                    then_block: BackendBlockId(1),
                    else_block: BackendBlockId(2),
                    span: actus::lexer::SourceSpan::new(0, 1),
                }],
            },
            BackendBlock {
                id: BackendBlockId(1),
                instructions: vec![BackendInstruction::Jump {
                    target: BackendBlockId(3),
                    span: actus::lexer::SourceSpan::new(1, 2),
                }],
            },
            BackendBlock {
                id: BackendBlockId(2),
                instructions: vec![BackendInstruction::Trap {
                    reason: "false parity branch".to_owned(),
                    span: actus::lexer::SourceSpan::new(2, 3),
                }],
            },
            BackendBlock {
                id: BackendBlockId(3),
                instructions: vec![BackendInstruction::Return {
                    value: None,
                    span: actus::lexer::SourceSpan::new(3, 4),
                }],
            },
        ],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![identity, entry, control] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("parity fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM parity object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift parity object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes).expect("Cranelift object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM parity objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift parity objects should link");
    let llvm_status =
        Command::new(&llvm_executable).status().expect("LLVM parity executable should run");
    let cranelift_status = Command::new(&cranelift_executable)
        .status()
        .expect("Cranelift parity executable should run");
    assert_eq!(llvm_status.code(), Some(0));
    assert_eq!(cranelift_status.code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(target_os = "linux")]
fn run_native_command(command: &mut std::process::Command) {
    let status = command.status().expect("native compiler should start");
    assert!(status.success(), "native compiler should succeed: {command:?}");
}

#[cfg(target_os = "linux")]
fn remove_fixture<const N: usize>(root: &Path, files: [&PathBuf; N]) {
    for file in files {
        let _ = fs::remove_file(file);
    }
    let _ = fs::remove_file(root.join("Actus.lock"));
    let _ = fs::remove_dir(root);
}

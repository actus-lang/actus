#![cfg(feature = "llvm")]

use actus::ast::Role;
#[cfg(feature = "cranelift")]
use actus::codegen::cranelift::emit_contract_module_object;
use actus::codegen::link_objects;
use actus::codegen::llvm::{emit_module, emit_object};
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendCaseMode, BackendCleanupArgument, BackendCleanupBridge,
    BackendCleanupOperand, BackendCleanupResolvedDescriptor, BackendCleanupResult,
    BackendEnumLayout, BackendExternal, BackendFailure, BackendFunction, BackendInstruction,
    BackendLayout, BackendModule, BackendParameter, BackendReturn, BackendReturnMode,
    BackendValueId, BackendValueType, LoweredCleanupInstruction, SymbolIdentity, SymbolKind,
    lower_scalar_program,
};
use actus::configuration::{CompilerConfiguration, NativeBackendConfiguration};
use actus::lexer::scan;
use actus::parser::parse;
use object::{Object, ObjectSection, ObjectSymbol};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(feature = "cranelift")]
#[test]
fn shared_lowered_module_is_consumed_by_llvm_and_cranelift() {
    let source = "verb main() -> Int { return 40 + 2; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("shared backend fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("source should lower through the shared contract");

    let llvm_ir = emit_module(&module).expect("LLVM should consume the shared module");
    assert!(llvm_ir.contains("define i32 @main()"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let cranelift_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should consume the shared module");
    let object = object::File::parse(cranelift_bytes.as_slice()).expect("object should parse");
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("main")));
}

#[cfg(feature = "cranelift")]
#[test]
fn both_backends_emit_user_defined_drop_performance_calls() {
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { return; } } verb inspect() { erg value: Counter = Counter { value: 7, }; return; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Drop performance fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Drop performance fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("inspect"))
        .expect("Drop performance fixture should lower through the shared contract");

    let llvm_ir = emit_module(&module).expect("LLVM should emit the user-defined Drop call");
    assert!(llvm_ir.contains("define void @actus_root__performance_drop__Drop_Counter"));
    assert!(llvm_ir.contains("call void @actus_root__performance_drop__Drop_Counter"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should emit the user-defined Drop call");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(
        object
            .symbols()
            .any(|symbol| symbol.name() == Ok("actus_root__performance_drop__Drop_Counter"))
    );
}

#[test]
fn both_backends_emit_scalar_aggregate_field_access() {
    let source = "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; return counter.value + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "aggregate field fixture should lex: {errors:?}");
    let program = parse(tokens).expect("aggregate field fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("aggregate field access should lower through the shared contract");

    let llvm_ir = emit_module(&module).expect("LLVM should emit aggregate field access");
    assert!(llvm_ir.contains("aggregate.field.load"));

    #[cfg(feature = "cranelift")]
    {
        let target = actus::target::TargetSpec::host().expect("host target should parse");
        let configuration = NativeBackendConfiguration::default();
        let object_bytes = emit_contract_module_object(&module, &configuration, &target)
            .expect("Cranelift should emit aggregate field access");
        let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
        assert!(object.symbols().any(|symbol| symbol.name() == Ok("main")));
    }
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_consumes_scalar_result_case_from_shared_lowering() {
    let source = "verb inspect(dat result: Result[Int, Int]) -> Int { return case dat result { Result.Ok(value) => value, Result.Err(error) => error, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Result fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Result fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("inspect"))
        .expect("scalar Result case should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should consume scalar Result extraction");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("inspect")));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_emits_scalar_result_constructor_to_return_slot() {
    let source = "verb main() -> Result[Int, Int] { return Result[Int, Int].Ok(7); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Result constructor should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Result constructor should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("Result constructor should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should emit scalar Result construction");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("main")));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_emits_region_result_constructor_to_return_slot() {
    let source = "verb wrap(dat region: Region[u32]) -> Result[Region[u32], Int] { return Result[Region[u32], Int].Ok(region); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Region Result fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Region Result fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("wrap"))
        .expect("Region Result constructor should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should emit a Region Result constructor");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("wrap")));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_consumes_region_result_case_from_shared_lowering() {
    let source = "verb inspect(dat result: Result[Region[u32], Int]) -> Int { return case dat result { Result.Ok(value) => 0, Result.Err(error) => error, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Region case fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Region case fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("inspect"))
        .expect("Region Result case should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should consume a Region Result case");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("inspect")));
}

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
fn emits_a_neutral_integer_constant_and_returns_it() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "constant").expect("valid symbol");
    let integer = BackendValueType::Integer { signed: false, width: 32 };
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: integer,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                id: BackendBlockId(0),
                instructions: vec![
                    BackendInstruction::Constant {
                        result: BackendValueId(0),
                        ty: integer,
                        value: 42,
                        span: actus::lexer::SourceSpan::new(0, 2),
                    },
                    BackendInstruction::Return {
                        value: Some(BackendValueId(0)),
                        span: actus::lexer::SourceSpan::new(2, 3),
                    },
                ],
            }],
            cleanup: Vec::new(),
        }],
    };

    let ir = emit_module(&module).expect("constant should lower to LLVM IR");
    assert!(ir.contains("ret i32 42"));
}

#[test]
fn emits_neutral_enum_discriminant_and_payload_extraction() {
    let enum_layout = BackendEnumLayout {
        layout: BackendLayout { size: 8, alignment: 4 },
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset: 4,
        payload: Some(BackendLayout { size: 4, alignment: 4 }),
        niche_pointer: false,
    };
    let enum_type = BackendValueType::Enum { layout: enum_layout };
    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "enum_extract").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "result".to_owned(),
            role: Role::Erg,
            ty: enum_type,
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::EnumDiscriminant {
                    result: BackendValueId(1),
                    subject: BackendValueId(0),
                    layout: enum_layout,
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::EnumPayload {
                    result: BackendValueId(2),
                    subject: BackendValueId(0),
                    layout: enum_layout,
                    payload_type: integer,
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(2)),
                    span: actus::lexer::SourceSpan::new(2, 3),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("LLVM should emit neutral enum extraction");
    assert!(ir.contains("load i32"));
    assert!(ir.contains("getelementptr i8"));
}

#[test]
fn emits_neutral_enum_payload_binding_with_ownership_mode() {
    let enum_layout = BackendEnumLayout {
        layout: BackendLayout { size: 8, alignment: 4 },
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset: 4,
        payload: Some(BackendLayout { size: 4, alignment: 4 }),
        niche_pointer: false,
    };
    let enum_type = BackendValueType::Enum { layout: enum_layout };
    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "enum_binding").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "result".to_owned(),
            role: Role::Erg,
            ty: enum_type,
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::EnumPayloadBinding {
                    result: BackendValueId(1),
                    subject: BackendValueId(0),
                    layout: enum_layout,
                    payload_type: integer,
                    mode: BackendCaseMode::Dat,
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
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("LLVM should emit enum payload binding");
    assert!(ir.contains("load i32"));
    assert!(ir.contains("getelementptr i8"));
}

#[test]
fn emits_region_payload_result_constructor_from_shared_lowering() {
    let source = "verb wrap(dat region: Region[u32]) -> Result[Region[u32], Int] { return Result[Region[u32], Int].Ok(region); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Region Result fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Region Result fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("Region Result source should lower through the neutral contract");
    emit_module(&module).expect("LLVM should consume the Region Result contract");
}

#[test]
fn emits_neutral_result_constructor_with_payload() {
    let enum_layout = BackendEnumLayout {
        layout: BackendLayout { size: 8, alignment: 4 },
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset: 4,
        payload: Some(BackendLayout { size: 4, alignment: 4 }),
        niche_pointer: false,
    };
    let enum_type = BackendValueType::Enum { layout: enum_layout };
    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "result_construct").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: enum_type,
            passing: BackendReturnMode::ReturnSlot { layout: enum_layout.layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Constant {
                    result: BackendValueId(0),
                    ty: integer,
                    value: 7,
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::EnumConstruct {
                    result: BackendValueId(1),
                    discriminant: 0,
                    payload: BackendValueId(0),
                    layout: enum_layout,
                    payload_type: integer,
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(2, 3),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("LLVM should emit neutral Result construction");
    assert!(ir.contains("store i32 7"));
    assert!(ir.contains("ret void"));
}

#[test]
fn emits_plain_result_case_from_shared_lowering() {
    let source = "verb produce() -> Result[Int, Int] { return Result[Int, Int].Ok(7); } verb main() -> Int { erg result = produce(); return case result { Result.Ok(_) => 0, Result.Err(_) => 1, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Result case fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Result case fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("Result case should lower through the shared contract");
    let ir = emit_module(&module).expect("LLVM should emit the shared Result case");
    assert!(ir.contains("br i1"));
    assert!(ir.contains("getelementptr i8"));
    assert!(ir.contains("ret i32 0"));
    assert!(ir.contains("ret i32 1"));
}

#[test]
fn emits_checked_runtime_integer_cast_from_shared_contract() {
    let source = "verb identity(erg value: Int) -> u32 { return value as u32; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("runtime cast fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("runtime cast should lower through the shared contract");
    let ir = emit_module(&module).expect("LLVM should emit checked runtime cast");
    assert!(ir.contains("actus.cast.trap"));
    assert!(ir.contains("trunc i64"));
}

#[test]
fn emits_signed_division_overflow_guard_for_runtime_operands() {
    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "checked_division").expect("valid symbol");
    let parameter = |name: &str| BackendParameter {
        name: name.to_owned(),
        role: Role::Erg,
        ty: integer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: vec![parameter("left"), parameter("right")],
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: integer,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                id: BackendBlockId(0),
                instructions: vec![
                    BackendInstruction::TrapIfZero {
                        value: BackendValueId(1),
                        reason: "division by zero".to_owned(),
                        span: actus::lexer::SourceSpan::new(0, 1),
                    },
                    BackendInstruction::TrapIfSignedDivisionOverflow {
                        left: BackendValueId(0),
                        right: BackendValueId(1),
                        width: 32,
                        reason: "signed division overflow".to_owned(),
                        span: actus::lexer::SourceSpan::new(1, 2),
                    },
                    BackendInstruction::IntegerBinary {
                        result: BackendValueId(2),
                        operation: actus::codegen::BackendIntegerBinaryOp::Divide,
                        left: BackendValueId(0),
                        right: BackendValueId(1),
                        ty: integer,
                        span: actus::lexer::SourceSpan::new(2, 3),
                    },
                    BackendInstruction::Return {
                        value: Some(BackendValueId(2)),
                        span: actus::lexer::SourceSpan::new(3, 4),
                    },
                ],
            }],
            cleanup: Vec::new(),
        }],
    };

    let ir = emit_module(&module).expect("runtime division guard should lower to LLVM IR");
    assert!(ir.contains("sdiv i32"));
    assert!(ir.contains("actus.division.trap"));
    assert!(ir.contains("actus.div.overflow"));
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
        runtime_pointer_return: false,
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
                    operand: None,
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
fn emits_region_runtime_cleanup_bridge_call() {
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "runtime_cleanup").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "region".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Pointer,
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
                BackendInstruction::Cleanup {
                    action: LoweredCleanupInstruction::DropBinding {
                        binding_index: 0,
                        name: "region".to_owned(),
                    },
                    operand: Some(BackendCleanupOperand {
                        value: BackendValueId(0),
                        layout: BackendLayout { size: 16, alignment: 8 },
                        field: None,
                        performance_symbol: None,
                        bridge: BackendCleanupBridge::RegionDrop,
                        argument: BackendCleanupArgument::Address,
                        result: BackendCleanupResult::Status,
                    }),
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
        .expect("region cleanup bridge should lower to an LLVM call");
    assert!(ir.contains("declare i32 @actus_region_drop(ptr)"));
    assert!(ir.contains("call i32 @actus_region_drop(ptr %0)"));
}

#[test]
fn emits_target_width_enum_runtime_cleanup_bridge_call() {
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "enum_cleanup").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Pointer,
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
                BackendInstruction::Cleanup {
                    action: LoweredCleanupInstruction::DropBinding {
                        binding_index: 0,
                        name: "value".to_owned(),
                    },
                    operand: Some(BackendCleanupOperand {
                        value: BackendValueId(0),
                        layout: BackendLayout { size: 24, alignment: 8 },
                        field: None,
                        performance_symbol: None,
                        bridge: BackendCleanupBridge::EnumDrop { size: 24 },
                        argument: BackendCleanupArgument::Address,
                        result: BackendCleanupResult::Ignored,
                    }),
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
        .expect("enum cleanup bridge should lower to an LLVM call");
    assert!(ir.contains("declare void @actus_enum_drop(ptr, i64)"));
    assert!(ir.contains("call void @actus_enum_drop(ptr %0, i64 24)"));
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
fn lowers_niche_option_failure_to_a_null_pointer() {
    let layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "niche_failure").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::NichePointerOption { layout },
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Option")
                        .expect("valid option symbol"),
                    discriminant: 0,
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
    let ir = emit_module(&BackendModule { externals: Vec::new(), functions: vec![function] })
        .expect("niche None should lower to a null pointer");
    assert!(ir.contains("ret ptr null"));
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
    let abi = actus::codegen::BackendDirectAggregateAbi {
        layout,
        parameter_fragments: vec![
            actus::codegen::BackendAggregateFragment {
                offset: 0,
                size: 8,
                alignment: 8,
                location: actus::codegen::BackendAggregateLocation::Register { index: 0 },
            },
            actus::codegen::BackendAggregateFragment {
                offset: 8,
                size: 8,
                alignment: 8,
                location: actus::codegen::BackendAggregateLocation::Register { index: 1 },
            },
        ],
        return_fragments: vec![
            actus::codegen::BackendAggregateFragment {
                offset: 0,
                size: 8,
                alignment: 8,
                location: actus::codegen::BackendAggregateLocation::Register { index: 0 },
            },
            actus::codegen::BackendAggregateFragment {
                offset: 8,
                size: 8,
                alignment: 8,
                location: actus::codegen::BackendAggregateLocation::Register { index: 1 },
            },
        ],
    };
    let aggregate = BackendValueType::Aggregate { layout };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "aggregate").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        parameters: vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: aggregate,
            passing: actus::codegen::BackendPassingMode::DirectAggregate { abi: abi.clone() },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::DirectAggregate { abi: abi.clone() },
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

#[test]
fn emits_an_arm32_thumb_object_for_the_cortex_m_profile() {
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
    let output =
        std::env::temp_dir().join(format!("actus-llvm-arm-object-{}.o", std::process::id()));
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object(&module, &target, &output).expect("ARM object should be emitted");
    let bytes = std::fs::read(&output).expect("ARM object should be readable");
    let object = object::File::parse(bytes.as_slice()).expect("ARM object should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    assert!(object.section_by_name(".text").is_some());
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("test__verb_main")));
    std::fs::remove_file(output).expect("ARM test object should be removed");
}

#[test]
fn emits_arm32_thumb_scalar_and_aggregate_call_fixtures() {
    let integer = BackendValueType::Integer { signed: false, width: 32 };
    let scalar_callee_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "scalar_callee").expect("scalar symbol");
    let scalar_caller_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "scalar_caller").expect("scalar symbol");
    let scalar_parameter = BackendParameter {
        name: "input".to_owned(),
        role: Role::Erg,
        ty: integer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let scalar_callee = BackendFunction {
        symbol: scalar_callee_symbol.clone(),
        parameters: vec![scalar_parameter.clone()],
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
    let scalar_caller = BackendFunction {
        symbol: scalar_caller_symbol,
        parameters: vec![scalar_parameter],
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
                    callee: scalar_callee_symbol,
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

    let aggregate_layout = actus::codegen::BackendLayout { size: 8, alignment: 4 };
    let aggregate_abi = actus::codegen::BackendDirectAggregateAbi {
        layout: aggregate_layout,
        parameter_fragments: arm_aggregate_register_fragments(),
        return_fragments: arm_aggregate_register_fragments(),
    };
    let aggregate_type = BackendValueType::Aggregate { layout: aggregate_layout };
    let aggregate_callee_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "aggregate_callee").expect("aggregate symbol");
    let aggregate_caller_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "aggregate_caller").expect("aggregate symbol");
    let aggregate_callee = BackendFunction {
        symbol: aggregate_callee_symbol.clone(),
        parameters: vec![BackendParameter {
            name: "input".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate {
                abi: aggregate_abi.clone(),
            },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi: aggregate_abi.clone() },
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
    let aggregate_caller = BackendFunction {
        symbol: aggregate_caller_symbol,
        parameters: vec![BackendParameter {
            name: "input".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate {
                abi: aggregate_abi.clone(),
            },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi: aggregate_abi },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: aggregate_callee_symbol,
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

    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![scalar_callee, scalar_caller, aggregate_callee, aggregate_caller],
    };
    let output =
        std::env::temp_dir().join(format!("actus-llvm-arm-call-fixtures-{}.o", std::process::id()));
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object(&module, &target, &output).expect("ARM call fixture object should be emitted");
    let bytes = std::fs::read(&output).expect("ARM call fixture object should be readable");
    let object =
        object::File::parse(bytes.as_slice()).expect("ARM call fixture object should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    let text = object.section_by_name(".text").expect("ARM call fixture should contain .text");
    assert!(text.align() >= 2, "Thumb code must have at least halfword alignment");
    let hosted_imports = ["abort", "free", "malloc", "memcpy", "printf"];
    for symbol in object.symbols().filter(|symbol| symbol.is_undefined()) {
        let name = symbol.name().expect("undefined ARM symbol should have a name");
        assert!(!hosted_imports.contains(&name), "ARM object imports hosted symbol {name}");
    }
    for symbol in [
        "arm__verb_scalar_5fcallee",
        "arm__verb_scalar_5fcaller",
        "arm__verb_aggregate_5fcallee",
        "arm__verb_aggregate_5fcaller",
    ] {
        assert!(object.symbols().any(|candidate| candidate.name() == Ok(symbol)));
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let disassembly = Command::new("llvm-objdump")
            .args(["-d", "--triple=thumbv7em-none-eabihf"])
            .arg(&output)
            .output()
            .expect("llvm-objdump should be available for ARM stack validation");
        assert!(disassembly.status.success(), "llvm-objdump failed for ARM call fixture");
        let disassembly =
            String::from_utf8(disassembly.stdout).expect("ARM disassembly should be UTF-8");
        assert_eq!(
            disassembly.matches("push	{r7, lr}").count(),
            2,
            "each ARM caller must preserve an 8-byte aligned call frame"
        );
        assert_eq!(
            disassembly.matches("pop	{r7, pc}").count(),
            2,
            "each ARM caller must restore its 8-byte aligned call frame"
        );
        assert_eq!(
            disassembly.matches("	bl	").count(),
            2,
            "each ARM caller must contain its direct call"
        );
    }
    if std::env::var_os("ACTUS_KEEP_ARM_CALL_OBJECT").is_none() {
        std::fs::remove_file(output).expect("ARM call fixture object should be removed");
    } else {
        eprintln!("kept ARM call fixture object at {}", output.display());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn links_a_minimal_arm32_thumb_image_with_explicit_startup_and_provider() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-llvm-arm-image-{}", std::process::id()));
    fs::create_dir_all(&root).expect("ARM image fixture directory should be created");
    let startup = root.join("startup.s");
    fs::write(
        &startup,
        ".syntax unified\n.cpu cortex-m7\n.thumb\n.section .isr_vector,\"a\",%progbits\n.section .text.Reset_Handler,\"ax\",%progbits\n.global Reset_Handler\n.type Reset_Handler,%function\n.thumb_func\nReset_Handler:\n  bl arm__verb_boot\n1: b 1b\n.global __aeabi_unwind_cpp_pr0\n.type __aeabi_unwind_cpp_pr0,%function\n.thumb_func\n__aeabi_unwind_cpp_pr0:\n  bx lr\n",
    )
    .expect("ARM startup source should be written");
    let linker_script = root.join("memory.x");
    fs::write(
        &linker_script,
        "ENTRY(Reset_Handler)\nMEMORY\n{\n  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 256K\n  RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 64K\n}\n_estack = ORIGIN(RAM) + LENGTH(RAM);\nSECTIONS\n{\n  .isr_vector ALIGN(4) : ALIGN(4) { LONG(_estack); LONG((Reset_Handler & ~1) + 1); KEEP(*(.isr_vector)) } > FLASH\n  .text : { *(.text*) *(.rodata*) } > FLASH\n  .data : { *(.data*) } > RAM AT > FLASH\n  .bss : { *(.bss*) *(COMMON) } > RAM\n}\n",
    )
    .expect("ARM linker script should be written");
    let startup_object = root.join("startup.o");
    let actus_object = root.join("boot.o");
    let image = root.join("image.elf");

    run_native_command(
        Command::new("clang")
            .args(["--target=thumbv7em-none-eabihf", "-mcpu=cortex-m7", "-mthumb", "-c"])
            .arg(&startup)
            .arg("-o")
            .arg(&startup_object),
    );

    let symbol = SymbolIdentity::new("arm", SymbolKind::Verb, "boot").expect("valid ARM symbol");
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
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object(&module, &target, &actus_object).expect("ARM Actus object should be emitted");

    run_native_command(
        Command::new("rust-lld")
            .args(["-flavor", "gnu", "-T"])
            .arg(&linker_script)
            .arg(&startup_object)
            .arg(&actus_object)
            .arg("-o")
            .arg(&image),
    );

    let bytes = fs::read(&image).expect("ARM image should be readable");
    let object = object::File::parse(bytes.as_slice()).expect("ARM image should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    let vector = object.section_by_name(".isr_vector").expect("vector section should exist");
    let text = object.section_by_name(".text").expect("text section should exist");
    assert!(vector.align() >= 4, "vector table must be word aligned");
    assert!(text.align() >= 2, "Thumb text must be halfword aligned");
    assert!((0x0800_0000..0x0804_0000).contains(&vector.address()));
    assert!((0x0800_0000..0x0804_0000).contains(&text.address()));
    let vector_data = vector.data().expect("vector section should contain bytes");
    assert_eq!(vector_data.len(), 8, "minimal vector table must contain two words");
    let initial_stack = u32::from_le_bytes(vector_data[0..4].try_into().expect("stack word"));
    let reset_vector = u32::from_le_bytes(vector_data[4..8].try_into().expect("reset word"));
    assert_eq!(initial_stack, 0x2001_0000, "vector table must point at the linker-defined stack");
    assert_ne!(reset_vector & 1, 0, "Reset_Handler vector must select Thumb state");
    let reset_address = object
        .symbols()
        .find(|symbol| symbol.name() == Ok("Reset_Handler"))
        .expect("Reset_Handler should be exported")
        .address();
    assert_eq!(
        u64::from(reset_vector & !1),
        reset_address & !1,
        "vector must target Reset_Handler"
    );
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("arm__verb_boot")));
    let relocation_count =
        object.sections().map(|section| section.relocations().count()).sum::<usize>();
    assert_eq!(relocation_count, 0, "linked image must not retain relocations");
    let hosted_imports = ["abort", "free", "malloc", "memcpy", "printf"];
    for symbol in object.symbols().filter(|symbol| symbol.is_undefined()) {
        let name = symbol.name().expect("undefined symbol should have a name");
        assert!(!hosted_imports.contains(&name), "image imports hosted symbol {name}");
    }
    assert!(object.symbols().all(|symbol| !symbol.is_undefined()));
    if std::env::var_os("ACTUS_KEEP_ARM_IMAGE").is_none() {
        remove_fixture(&root, [&startup, &linker_script, &startup_object, &actus_object, &image]);
    } else {
        eprintln!("kept ARM image fixture at {}", image.display());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn links_an_explicit_external_c_provider_through_the_llvm_arm_path() {
    use std::process::Command;

    let root =
        std::env::temp_dir().join(format!("actus-llvm-arm-c-provider-{}", std::process::id()));
    fs::create_dir_all(&root).expect("external provider fixture directory should be created");
    let provider_source = root.join("provider.c");
    fs::write(&provider_source, "int runtime__extern_increment(void) { return 7; }\n")
        .expect("external C provider source should be written");
    let startup = root.join("startup.s");
    fs::write(
        &startup,
        ".syntax unified\n.cpu cortex-m7\n.thumb\n.section .text.Reset_Handler,\"ax\",%progbits\n.global Reset_Handler\n.type Reset_Handler,%function\n.thumb_func\nReset_Handler:\n  bl arm__verb_boot\n1: b 1b\n.global __aeabi_unwind_cpp_pr0\n.type __aeabi_unwind_cpp_pr0,%function\n.thumb_func\n__aeabi_unwind_cpp_pr0:\n  bx lr\n",
    )
    .expect("external provider startup source should be written");
    let linker_script = root.join("memory.x");
    fs::write(
        &linker_script,
        "ENTRY(Reset_Handler)\nMEMORY\n{\n  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 256K\n  RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 64K\n}\n_estack = ORIGIN(RAM) + LENGTH(RAM);\nSECTIONS\n{\n  .isr_vector ALIGN(4) : ALIGN(4) { LONG(_estack); LONG((Reset_Handler & ~1) + 1); KEEP(*(.isr_vector)) } > FLASH\n  .text : { *(.text*) *(.rodata*) } > FLASH\n  .data : { *(.data*) } > RAM AT > FLASH\n  .bss : { *(.bss*) *(COMMON) } > RAM\n}\n",
    )
    .expect("external provider linker script should be written");
    let provider_object = root.join("provider.o");
    let startup_object = root.join("startup.o");
    let actus_object = root.join("boot.o");
    let image = root.join("image.elf");

    run_native_command(
        Command::new("clang")
            .args([
                "--target=thumbv7em-none-eabihf",
                "-mcpu=cortex-m7",
                "-mthumb",
                "-ffreestanding",
                "-fno-builtin",
                "-nostdlib",
                "-c",
            ])
            .arg(&provider_source)
            .arg("-o")
            .arg(&provider_object),
    );
    run_native_command(
        Command::new("clang")
            .args(["--target=thumbv7em-none-eabihf", "-mcpu=cortex-m7", "-mthumb", "-c"])
            .arg(&startup)
            .arg("-o")
            .arg(&startup_object),
    );

    let external_symbol = SymbolIdentity::new("runtime", SymbolKind::External, "increment")
        .expect("valid external symbol");
    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let module = BackendModule {
        externals: vec![BackendExternal {
            symbol: external_symbol.clone(),
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: integer,
                passing: BackendReturnMode::Direct,
            },
            runtime_pointer_return: false,
        }],
        functions: vec![BackendFunction {
            symbol: SymbolIdentity::new("arm", SymbolKind::Verb, "boot")
                .expect("valid boot symbol"),
            parameters: Vec::new(),
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
                        callee: external_symbol,
                        arguments: Vec::new(),
                        result: Some(BackendValueId(0)),
                        span: actus::lexer::SourceSpan::new(0, 1),
                    },
                    BackendInstruction::Return {
                        value: Some(BackendValueId(0)),
                        span: actus::lexer::SourceSpan::new(1, 2),
                    },
                ],
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object(&module, &target, &actus_object)
        .expect("Actus object with an external provider call should be emitted");

    run_native_command(
        Command::new("rust-lld")
            .args(["-flavor", "gnu", "-T"])
            .arg(&linker_script)
            .arg(&startup_object)
            .arg(&actus_object)
            .arg(&provider_object)
            .arg("-o")
            .arg(&image),
    );

    let bytes = fs::read(&image).expect("external provider image should be readable");
    let object =
        object::File::parse(bytes.as_slice()).expect("external provider image should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("runtime__extern_increment")));
    assert!(object.symbols().any(|symbol| symbol.name() == Ok("arm__verb_boot")));
    let undefined = object
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .map(|symbol| symbol.name().unwrap_or("<invalid>").to_owned())
        .collect::<Vec<_>>();
    assert!(
        undefined.is_empty(),
        "linked external provider image has undefined symbols: {undefined:?}"
    );
    assert_eq!(
        object.sections().map(|section| section.relocations().count()).sum::<usize>(),
        0,
        "linked external provider image must not retain relocations"
    );
    remove_fixture(
        &root,
        [
            &provider_source,
            &startup,
            &linker_script,
            &provider_object,
            &startup_object,
            &actus_object,
            &image,
        ],
    );
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
fn compares_llvm_and_cranelift_on_niche_option_none_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-niche-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("niche parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"niche_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("niche parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern void *test__verb_none(void); int main(void) { return test__verb_none() == 0 ? 0 : 1; }\n",
    )
    .expect("niche parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let function = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "none").expect("valid symbol"),
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::NichePointerOption { layout },
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Option")
                        .expect("valid option symbol"),
                    discriminant: 0,
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
    let module = BackendModule { externals: Vec::new(), functions: vec![function] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("niche parity fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM niche object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift niche object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift niche object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM niche objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift niche objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
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

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_niche_option_some_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-niche-some-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("niche Some parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"niche_some_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("niche Some parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern void *test__verb_some(void *); int main(void) { int value = 42; return test__verb_some(&value) == &value ? 0 : 1; }\n",
    )
    .expect("niche Some parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let pointer = BackendParameter {
        name: "payload".to_owned(),
        role: Role::Erg,
        ty: BackendValueType::Pointer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let function = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "some").expect("valid symbol"),
        parameters: vec![pointer],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::NichePointerOption { layout },
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
                    payload: Some(BackendValueId(0)),
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
    let module = BackendModule { externals: Vec::new(), functions: vec![function] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("niche Some parity fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM niche Some object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift niche Some object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift niche Some object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM niche Some objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift niche Some objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
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

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_runtime_cleanup_order_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-cleanup-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("cleanup parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"cleanup_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("cleanup parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nstatic int order; static int status; static void *expected_buffer; static void *expected_region; static void *expected_enum; static uint64_t expected_enum_size; void actus_buffer_drop(void *value) { if (value == expected_buffer) order = order * 10 + 1; else order = -1; } int32_t actus_region_drop(void *value) { if (value == expected_region) { order = order * 10 + 2; status = 0; } else { order = -1; status = -1; } return status; } void actus_enum_drop(void *value, uint64_t size) { if (value == expected_enum && size == expected_enum_size) order = order * 10 + 3; else order = -1; } extern void test__verb_cleanup(void *, void *, void *); int main(void) { uint64_t buffer = 1; uint64_t region = 2; uint64_t enum_value[2] = { 3, 4 }; expected_buffer = &buffer; expected_region = &region; expected_enum = enum_value; expected_enum_size = sizeof(enum_value); test__verb_cleanup(&buffer, &region, enum_value); return order == 123 && status == 0 ? 0 : 1; }\n",
    )
    .expect("cleanup parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let pointer_layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let pointer = BackendValueType::Pointer;
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "cleanup").expect("symbol");
    let parameter = |name: &str| BackendParameter {
        name: name.to_owned(),
        role: Role::Erg,
        ty: pointer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let buffer_descriptor = BackendCleanupResolvedDescriptor {
        descriptor: actus::codegen::BackendCleanupBridgeDescriptor {
            bridge: BackendCleanupBridge::BufferDrop,
            argument: BackendCleanupArgument::Value,
            result: BackendCleanupResult::Ignored,
        },
        layout: pointer_layout,
        field: None,
        performance_symbol: None,
    };
    let region_descriptor = BackendCleanupResolvedDescriptor {
        descriptor: actus::codegen::BackendCleanupBridgeDescriptor {
            bridge: BackendCleanupBridge::RegionDrop,
            argument: BackendCleanupArgument::Address,
            result: BackendCleanupResult::Status,
        },
        layout: pointer_layout,
        field: None,
        performance_symbol: None,
    };
    let enum_descriptor = BackendCleanupResolvedDescriptor {
        descriptor: actus::codegen::BackendCleanupBridgeDescriptor {
            bridge: BackendCleanupBridge::EnumDrop { size: 16 },
            argument: BackendCleanupArgument::Address,
            result: BackendCleanupResult::Ignored,
        },
        layout: actus::codegen::BackendLayout { size: 16, alignment: 8 },
        field: None,
        performance_symbol: None,
    };
    buffer_descriptor.validate().expect("buffer descriptor should validate");
    region_descriptor.validate().expect("region descriptor should validate");
    enum_descriptor.validate().expect("enum descriptor should validate");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: vec![parameter("buffer"), parameter("region"), parameter("enum_value")],
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
                        action: LoweredCleanupInstruction::DropBinding {
                            binding_index: 0,
                            name: "buffer".to_owned(),
                        },
                        operand: Some(buffer_descriptor.operand(BackendValueId(0))),
                        span: actus::lexer::SourceSpan::new(0, 1),
                    },
                    BackendInstruction::Cleanup {
                        action: LoweredCleanupInstruction::DropBinding {
                            binding_index: 1,
                            name: "region".to_owned(),
                        },
                        operand: Some(region_descriptor.operand(BackendValueId(1))),
                        span: actus::lexer::SourceSpan::new(1, 2),
                    },
                    BackendInstruction::Cleanup {
                        action: LoweredCleanupInstruction::DropBinding {
                            binding_index: 2,
                            name: "enum_value".to_owned(),
                        },
                        operand: Some(enum_descriptor.operand(BackendValueId(2))),
                        span: actus::lexer::SourceSpan::new(2, 3),
                    },
                    BackendInstruction::Return {
                        value: None,
                        span: actus::lexer::SourceSpan::new(3, 4),
                    },
                ],
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("cleanup parity configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM cleanup object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift cleanup object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift cleanup object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM cleanup objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift cleanup objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
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

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_direct_register_aggregate_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-direct-aggregate-{}", std::process::id()));
    fs::create_dir_all(&root).expect("direct aggregate fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"direct_aggregate_fixture\"\nversion = \"0.1.0\"\n")
        .expect("direct aggregate manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\ntypedef struct { uint64_t first; uint64_t second; } Pair; extern Pair test__verb_caller(Pair); int main(void) { Pair input = {40, 2}; Pair output = test__verb_caller(input); return output.first == 40 && output.second == 2 ? 0 : 1; }\n",
    )
    .expect("direct aggregate entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let abi = actus::codegen::BackendDirectAggregateAbi {
        layout,
        parameter_fragments: aggregate_register_fragments(),
        return_fragments: aggregate_register_fragments(),
    };
    let aggregate_type = BackendValueType::Aggregate { layout };
    let aggregate_function = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "aggregate").expect("symbol"),
        parameters: vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate { abi: abi.clone() },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi: abi.clone() },
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
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "caller").expect("symbol"),
        parameters: vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate { abi: abi.clone() },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: SymbolIdentity::new("test", SymbolKind::Verb, "aggregate")
                        .expect("aggregate callee symbol"),
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
    let module =
        BackendModule { externals: Vec::new(), functions: vec![aggregate_function, caller] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("direct aggregate fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM direct aggregate object should emit");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift direct aggregate object should emit");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift direct aggregate object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM direct aggregate objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift direct aggregate objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
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

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_result_return_slot_call_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-result-call-{}", std::process::id()));
    fs::create_dir_all(&root).expect("Result call fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"result_call_fixture\"\nversion = \"0.1.0\"\n")
        .expect("Result call manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nextern int32_t test__verb_caller(void); int main(void) { return test__verb_caller() == 0 ? 0 : 1; }\n",
    )
    .expect("Result call entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let result_layout = BackendLayout { size: 16, alignment: 8 };
    let payload_type = BackendValueType::Integer { signed: true, width: 32 };
    let result_type = BackendValueType::Enum {
        layout: BackendEnumLayout {
            layout: result_layout,
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset: 8,
            payload: Some(BackendLayout { size: 4, alignment: 4 }),
            niche_pointer: false,
        },
    };
    let callee_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "produce").expect("callee symbol");
    let caller_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "caller").expect("caller symbol");
    let callee = BackendFunction {
        symbol: callee_symbol.clone(),
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: result_type,
            passing: BackendReturnMode::ReturnSlot { layout: result_layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Constant {
                    result: BackendValueId(0),
                    ty: payload_type,
                    value: 7,
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::EnumConstruct {
                    result: BackendValueId(1),
                    discriminant: 0,
                    payload: BackendValueId(0),
                    layout: match result_type {
                        BackendValueType::Enum { layout } => layout,
                        _ => unreachable!(),
                    },
                    payload_type,
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(2, 3),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let caller = BackendFunction {
        symbol: caller_symbol,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: payload_type,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: callee_symbol,
                    arguments: Vec::new(),
                    result: Some(BackendValueId(2)),
                    span: actus::lexer::SourceSpan::new(3, 4),
                },
                BackendInstruction::EnumDiscriminant {
                    result: BackendValueId(3),
                    subject: BackendValueId(2),
                    layout: match result_type {
                        BackendValueType::Enum { layout } => layout,
                        _ => unreachable!(),
                    },
                    span: actus::lexer::SourceSpan::new(4, 5),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(3)),
                    span: actus::lexer::SourceSpan::new(5, 6),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![callee, caller] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("Result call fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM Result call object should emit");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift Result call object should emit");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift Result call object should be written");
    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM Result call objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift Result call objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
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

#[cfg(all(target_os = "linux", feature = "cranelift"))]
fn aggregate_register_fragments() -> Vec<actus::codegen::BackendAggregateFragment> {
    vec![
        actus::codegen::BackendAggregateFragment {
            offset: 0,
            size: 8,
            alignment: 8,
            location: actus::codegen::BackendAggregateLocation::Register { index: 0 },
        },
        actus::codegen::BackendAggregateFragment {
            offset: 8,
            size: 8,
            alignment: 8,
            location: actus::codegen::BackendAggregateLocation::Register { index: 1 },
        },
    ]
}

fn arm_aggregate_register_fragments() -> Vec<actus::codegen::BackendAggregateFragment> {
    vec![
        actus::codegen::BackendAggregateFragment {
            offset: 0,
            size: 4,
            alignment: 4,
            location: actus::codegen::BackendAggregateLocation::Register { index: 0 },
        },
        actus::codegen::BackendAggregateFragment {
            offset: 4,
            size: 4,
            alignment: 4,
            location: actus::codegen::BackendAggregateLocation::Register { index: 1 },
        },
    ]
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_the_same_indirect_aggregate_copy_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-aggregate-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("aggregate parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"aggregate_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("aggregate parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nextern void test__verb_copy(void *, const void *); int main(void) { uint64_t source[2] = { 0x1122334455667788ULL, 0x99aabbccddeeff00ULL }; uint64_t destination[2] = { 0, 0 }; test__verb_copy(destination, source); return destination[0] == source[0] && destination[1] == source[1] ? 0 : 1; }\n",
    )
    .expect("aggregate parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "copy").expect("copy symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: vec![
                BackendParameter {
                    name: "destination".to_owned(),
                    role: Role::Ins,
                    ty: BackendValueType::Aggregate { layout },
                    passing: actus::codegen::BackendPassingMode::ExclusiveLoan { layout },
                },
                BackendParameter {
                    name: "source".to_owned(),
                    role: Role::Abs,
                    ty: BackendValueType::Aggregate { layout },
                    passing: actus::codegen::BackendPassingMode::Borrowed { layout },
                },
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
                        destination: BackendValueId(0),
                        source: BackendValueId(1),
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
        }],
    };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("aggregate parity configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM aggregate object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift aggregate object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift aggregate object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM aggregate objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift aggregate objects should link");
    let llvm_status =
        Command::new(&llvm_executable).status().expect("LLVM aggregate executable should run");
    let cranelift_status = Command::new(&cranelift_executable)
        .status()
        .expect("Cranelift aggregate executable should run");
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

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_the_same_return_slot_failure_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-failure-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("failure parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"failure_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("failure parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nextern void test__verb_fail(void *, int32_t); int main(void) { uint64_t slot[2] = { 0, 0 }; test__verb_fail(slot, 0x12345678); uint32_t *words = (uint32_t *)slot; return words[0] == 7U && words[2] == 0x12345678U ? 0 : 1; }\n",
    )
    .expect("failure parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let aggregate_layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let enum_layout = BackendEnumLayout {
        layout: aggregate_layout,
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset: 8,
        payload: Some(actus::codegen::BackendLayout { size: 4, alignment: 4 }),
        niche_pointer: false,
    };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "fail").expect("fail symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: vec![BackendParameter {
                name: "payload".to_owned(),
                role: Role::Erg,
                ty: BackendValueType::Integer { signed: true, width: 32 },
                passing: actus::codegen::BackendPassingMode::Direct,
            }],
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: BackendValueType::Aggregate { layout: aggregate_layout },
                passing: BackendReturnMode::ReturnSlot { layout: aggregate_layout },
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Fail {
                    failure: BackendFailure {
                        result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Result")
                            .expect("result symbol"),
                        discriminant: 7,
                        payload: Some(BackendValueId(0)),
                        layout: enum_layout,
                    },
                    span: actus::lexer::SourceSpan::new(0, 1),
                }],
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("failure parity configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM failure object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift failure object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift failure object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM failure objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift failure objects should link");
    let llvm_status =
        Command::new(&llvm_executable).status().expect("LLVM failure executable should run");
    let cranelift_status = Command::new(&cranelift_executable)
        .status()
        .expect("Cranelift failure executable should run");
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

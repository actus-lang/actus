use super::*;

#[test]
fn emits_a_valid_unit_function_as_llvm_ir() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: Vec::new(),
            }],
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
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: integer,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::EnumPayloadBinding {
                    result: BackendValueId(1),
                    subject: BackendValueId(0),
                    layout: enum_layout,
                    payload_type: integer,
                    payload_offset: 0,
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
        hosted_entry_return: false,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: enum_type,
            passing: BackendReturnMode::ReturnSlot { layout: enum_layout.layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
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
                    payload: Some(BackendValueId(0)),
                    layout: enum_layout,
                    payload_type: Some(integer),
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
            hosted_entry_return: false,
            parameters: vec![parameter("left"), parameter("right")],
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: integer,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
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
        hosted_entry_return: false,
        parameters: vec![parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
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
        hosted_entry_return: false,
        parameters: vec![parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
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
fn emits_block_parameter_phi_from_typed_edge_arguments() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "phi").expect("valid symbol");
    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let function = BackendFunction {
        symbol,
        hosted_entry_return: false,
        parameters: vec![BackendParameter {
            name: "condition".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Bool,
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: vec![
                    BackendInstruction::Constant {
                        result: BackendValueId(1),
                        ty: integer,
                        value: 41,
                        span: actus::lexer::SourceSpan::new(0, 1),
                    },
                    BackendInstruction::Branch {
                        condition: BackendValueId(0),
                        then_block: BackendBlockId(1),
                        else_block: BackendBlockId(2),
                        then_arguments: Vec::new(),
                        else_arguments: Vec::new(),
                        span: actus::lexer::SourceSpan::new(1, 2),
                    },
                ],
            },
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(1),
                instructions: vec![BackendInstruction::Jump {
                    target: BackendBlockId(3),
                    arguments: vec![BackendEdgeArgument {
                        value: BackendValueId(1),
                        ty: integer,
                        span: actus::lexer::SourceSpan::new(2, 3),
                    }],
                    span: actus::lexer::SourceSpan::new(2, 3),
                }],
            },
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(2),
                instructions: vec![BackendInstruction::Jump {
                    target: BackendBlockId(3),
                    arguments: vec![BackendEdgeArgument {
                        value: BackendValueId(1),
                        ty: integer,
                        span: actus::lexer::SourceSpan::new(3, 4),
                    }],
                    span: actus::lexer::SourceSpan::new(3, 4),
                }],
            },
            BackendBlock {
                parameters: vec![BackendBlockParameter {
                    value: BackendValueId(2),
                    ty: integer,
                    span: actus::lexer::SourceSpan::new(4, 5),
                }],
                id: BackendBlockId(3),
                instructions: vec![BackendInstruction::Return {
                    value: Some(BackendValueId(2)),
                    span: actus::lexer::SourceSpan::new(5, 6),
                }],
            },
        ],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![function] };
    let ir = emit_module(&module).expect("LLVM should lower typed edge arguments to PHI");
    assert!(ir.contains("phi i32"));
    assert!(ir.contains("[ 41, %bb1 ]"));
    assert!(ir.contains("[ 41, %bb2 ]"));
    #[cfg(feature = "cranelift")]
    {
        let target = actus::target::TargetSpec::host().expect("host target should parse");
        let bytes =
            emit_contract_module_object(&module, &NativeBackendConfiguration::default(), &target)
                .expect("Cranelift should lower typed edge arguments to block parameters");
        let object = object::File::parse(bytes.as_slice()).expect("object should parse");
        assert!(object.symbols().any(|symbol| symbol.name().is_ok()));
    }
}

#[test]
fn emits_scalar_branch_jump_and_trap_terminators() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "control").expect("valid symbol");
    let function = BackendFunction {
        symbol,
        hosted_entry_return: false,
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
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Branch {
                    condition: BackendValueId(0),
                    then_block: BackendBlockId(1),
                    else_block: BackendBlockId(2),
                    then_arguments: Vec::new(),
                    else_arguments: Vec::new(),
                    span: actus::lexer::SourceSpan::new(0, 1),
                }],
            },
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(1),
                instructions: vec![BackendInstruction::Jump {
                    target: BackendBlockId(3),
                    arguments: Vec::new(),
                    span: actus::lexer::SourceSpan::new(1, 2),
                }],
            },
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(2),
                instructions: vec![BackendInstruction::Trap {
                    reason: "false branch".to_owned(),
                    span: actus::lexer::SourceSpan::new(2, 3),
                }],
            },
            BackendBlock {
                parameters: Vec::new(),
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
        hosted_entry_return: false,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
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
        hosted_entry_return: false,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::NichePointerOption { layout, payload_layout: None },
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
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
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Float { width: 32 },
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: Vec::new(),
            }],
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
        hosted_entry_return: false,
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
            parameters: Vec::new(),
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
        hosted_entry_return: false,
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
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: Vec::new(),
        }],
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
        hosted_entry_return: false,
        parameters: vec![parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::ReturnSlot { layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
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
        hosted_entry_return: false,
        parameters: vec![parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
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

#![cfg(feature = "llvm")]

use actus::ast::Role;
use actus::codegen::llvm::emit_module;
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendFunction, BackendInstruction, BackendModule,
    BackendParameter, BackendReturn, BackendReturnMode, BackendValueId, BackendValueType,
    SymbolIdentity, SymbolKind,
};

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

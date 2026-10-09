#![cfg(feature = "llvm")]

use actus::codegen::llvm::emit_module;
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendFunction, BackendModule, BackendReturn, BackendReturnMode,
    BackendValueType, SymbolIdentity, SymbolKind,
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

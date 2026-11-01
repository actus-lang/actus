use super::*;

#[test]
fn llvm_emits_unit_enum_equality() {
    let source = "enum State { Ready, Busy, } verb main(erg left: State, erg right: State) -> Bool { return left == right; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "enum equality fixture should lex: {errors:?}");
    let program = parse(tokens).expect("enum equality fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("enum equality should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should consume enum equality");
    assert!(llvm_ir.contains("icmp eq"));
}

#[test]
fn llvm_emits_non_native_integer_widths() {
    let source = "verb narrow(erg value: u1) -> u1 { return value; } verb wide(erg value: u128) -> u128 { return value; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "non-native integer fixture should lex: {errors:?}");
    let program = parse(tokens).expect("non-native integer fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("narrow"))
        .expect("non-native integer widths should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should represent all Actus integer widths");
    assert!(llvm_ir.contains("define i1 @narrow"));
    assert!(llvm_ir.contains("define i128") && llvm_ir.contains("wide"));
}

#[test]
fn llvm_rejects_invalid_integer_width_with_stable_error() {
    let invalid = BackendValueType::Integer { signed: false, width: 129 };
    let symbol = SymbolIdentity::new("actus_root", SymbolKind::Verb, "invalid_width")
        .expect("fixture symbol should be valid");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: invalid,
                passing: BackendReturnMode::Direct,
            },
            hosted_entry_return: false,
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                id: BackendBlockId(0),
                parameters: Vec::new(),
                instructions: vec![BackendInstruction::Return {
                    value: None,
                    span: actus::lexer::SourceSpan::new(0, 1),
                }],
            }],
            cleanup: Vec::new(),
        }],
    };
    let error = emit_module(&module).expect_err("invalid integer width must be rejected");
    assert_eq!(
        error.to_string(),
        "LLVM type lowering rejects invalid integer width 129; expected 1..=128"
    );
}

#[test]
fn llvm_emits_recursive_direct_call() {
    let source = "verb countdown(erg value: Int) -> Int { if value == 0 { return 0; } erg next: Int = value - 1; return countdown(value: erg next); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "recursive fixture should lex: {errors:?}");
    let program = parse(tokens).expect("recursive fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("countdown"))
        .expect("recursive call should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should emit recursive calls");
    assert!(llvm_ir.contains("call i32 @countdown"));
}

#[test]
fn llvm_zero_float_policy_rejects_generated_float_ir() {
    let symbol =
        SymbolIdentity::new("runtime", SymbolKind::External, "__addsf3").expect("valid symbol");
    let module = BackendModule {
        externals: vec![BackendExternal {
            symbol,
            parameters: vec![BackendParameter {
                name: "value".to_owned(),
                role: Role::Erg,
                ty: BackendValueType::Integer { signed: true, width: 32 },
                passing: BackendPassingMode::Direct,
            }],
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Integer { signed: true, width: 32 },
                passing: BackendReturnMode::Direct,
            },
            runtime_pointer_return: false,
        }],
        functions: Vec::new(),
    };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let error = emit_object_bytes_with_configuration_and_float_policy(&module, &target, true, true)
        .expect_err("LLVM zero-float policy must reject generated float IR");
    assert!(error.to_string().contains("zero-float verification"));
}

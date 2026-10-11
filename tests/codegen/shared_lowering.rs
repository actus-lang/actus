use actus::codegen::lower_scalar_program;
use actus::codegen::{
    BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
};

#[test]
fn lowers_dat_and_abs_result_payload_bindings_with_explicit_modes() {
    for (mode, expected) in [
        ("dat", actus::codegen::BackendCaseMode::Dat),
        ("abs", actus::codegen::BackendCaseMode::Abs),
    ] {
        let source = format!(
            "verb main() -> Int {{ erg result = Result[Int, Int].Ok(7); return case {mode} result {{ Result.Ok(value) => value, Result.Err(_) => 0, }}; }}"
        );
        let (tokens, errors) = actus::lexer::scan(&source);
        assert!(errors.is_empty(), "Result {mode} fixture should lex: {errors:?}");
        let program = actus::parser::parse(tokens).expect("Result binding fixture should parse");
        let module = actus::codegen::lower_scalar_program("actus_root", &program, None)
            .expect("Result payload binding should lower through the neutral contract");
        assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(instruction, BackendInstruction::EnumPayloadBinding { mode, .. } if *mode == expected)
        ));
    }
}

#[test]
fn lowers_result_constructor_with_region_payload_layout() {
    let source = "verb wrap(dat region: Region[u32]) -> Result[Region[u32], Int] { return Result[Region[u32], Int].Ok(region); }";
    let (tokens, errors) = actus::lexer::scan(source);
    assert!(errors.is_empty(), "Region Result fixture should lex: {errors:?}");
    let program = actus::parser::parse(tokens).expect("Region Result fixture should parse");
    let module = actus::codegen::lower_scalar_program("actus_root", &program, None)
        .expect("Region Result constructor should lower through the neutral contract");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::EnumConstruct {
                payload_type: actus::codegen::BackendValueType::Region { .. },
                ..
            }
        )
    ));
}

#[test]
fn derives_nested_struct_layouts_in_the_neutral_signature_contract() {
    let source = "struct Inner { erg first: u32, abs view: Buffer, } struct Outer { erg inner: Inner, erg tail: u16, } verb inspect(erg value: Outer) { return; }";
    let (tokens, errors) = actus::lexer::scan(source);
    assert!(errors.is_empty(), "struct layout fixture should lex: {errors:?}");
    let program = actus::parser::parse(tokens).expect("struct layout fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("nested struct signature should lower through the neutral contract");
    let parameter = &module.functions[0].parameters[0];
    assert!(matches!(
        parameter.ty,
        actus::codegen::BackendValueType::Aggregate {
            layout: actus::codegen::BackendLayout { size: 24, alignment: 8 }
        }
    ));
    assert!(matches!(
        parameter.passing,
        actus::codegen::BackendPassingMode::Indirect {
            layout: actus::codegen::BackendLayout { size: 24, alignment: 8 }
        }
    ));
}

#[test]
fn records_user_defined_drop_with_a_neutral_performance_bridge_symbol() {
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { return; } } verb inspect(erg value: Counter) { return; }";
    let (tokens, errors) = actus::lexer::scan(source);
    assert!(errors.is_empty(), "Drop performance fixture should lex: {errors:?}");
    let program = actus::parser::parse(tokens).expect("Drop performance fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("user-defined Drop should lower to a typed performance bridge descriptor");
    assert_eq!(module.functions.len(), 2);
    assert!(
        module.functions.iter().any(
            |function| function.symbol.as_str() == "actus_root__performance_drop__Drop_Counter"
        )
    );
    let descriptor = module.functions[0]
        .cleanup
        .iter()
        .flat_map(|plan| plan.resolved_bridges.iter().flatten())
        .next()
        .expect("Drop cleanup should have a resolved descriptor");
    assert_eq!(descriptor.descriptor.bridge, actus::codegen::BackendCleanupBridge::PerformanceDrop);
    assert_eq!(descriptor.layout.size, 4);
    assert_eq!(
        descriptor.performance_symbol.as_ref().map(|symbol| symbol.as_str()),
        Some("actus_root__performance_drop__Drop_Counter")
    );
}

#[test]
fn lowers_explicit_scalar_drop_without_backend_cleanup() {
    let source = "verb main() -> Int { erg value = 41; drop(value); return 42; }";
    let (tokens, errors) = actus::lexer::scan(source);
    assert!(errors.is_empty(), "explicit scalar drop fixture should lex: {errors:?}");
    let program = actus::parser::parse(tokens).expect("explicit scalar drop fixture should parse");
    let module = actus::codegen::lower_scalar_program("actus_root", &program, None)
        .expect("scalar drop should lower through the neutral contract");
    let instructions = module.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .collect::<Vec<_>>();
    assert!(
        instructions
            .iter()
            .all(|instruction| { !matches!(instruction, BackendInstruction::Cleanup { .. }) })
    );
    assert!(instructions.iter().any(|instruction| {
        matches!(instruction, BackendInstruction::Return { value: Some(_), .. })
    }));
}

#[test]
fn preserves_distinct_result_payload_types_for_case_bindings() {
    let source = "verb inspect(dat result: Result[Region[u32], Int]) -> Int { return case dat result { Result.Ok(value) => 0, Result.Err(error) => error, }; }";
    let (tokens, errors) = actus::lexer::scan(source);
    assert!(errors.is_empty(), "mixed Result fixture should lex: {errors:?}");
    let program = actus::parser::parse(tokens).expect("mixed Result fixture should parse");
    let module = actus::codegen::lower_scalar_program("actus_root", &program, None)
        .expect("mixed Result case should lower through the neutral contract");
    let bindings = module.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction {
            BackendInstruction::EnumPayloadBinding { payload_type, .. } => Some(payload_type),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(bindings.iter().any(|payload_type| {
        matches!(payload_type, actus::codegen::BackendValueType::Region { .. })
    }));
    assert!(bindings.iter().any(|payload_type| {
        matches!(
            payload_type,
            actus::codegen::BackendValueType::Integer { signed: true, width: 32 }
        )
    }));
}

#[test]
fn preserves_result_payload_types_for_call_results() {
    let source = "verb produce() -> Result[Region[u32], Int] { return Result[Region[u32], Int].Err(7); } verb inspect() -> Int { erg result = produce(); return case dat result { Result.Ok(value) => 0, Result.Err(error) => error, }; }";
    let (tokens, errors) = actus::lexer::scan(source);
    assert!(errors.is_empty(), "call Result fixture should lex: {errors:?}");
    let program = actus::parser::parse(tokens).expect("call Result fixture should parse");
    let module = actus::codegen::lower_scalar_program("actus_root", &program, None)
        .expect("call Result case should lower through the neutral contract");
    let inspect = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("inspect"))
        .expect("inspect function should be present");
    let bindings = inspect
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction {
            BackendInstruction::EnumPayloadBinding { payload_type, .. } => Some(payload_type),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(bindings.iter().any(|payload_type| {
        matches!(payload_type, actus::codegen::BackendValueType::Region { .. })
    }));
    assert!(bindings.iter().any(|payload_type| {
        matches!(
            payload_type,
            actus::codegen::BackendValueType::Integer { signed: true, width: 32 }
        )
    }));
}
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn lowers_scalar_return_from_parsed_program() {
    let source = "verb main() -> Int { return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar fixture should parse");

    let module =
        lower_scalar_program("actus_root", &program, None).expect("scalar fixture should lower");
    assert_eq!(module.functions.len(), 1);
    assert!(
        module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(instruction, BackendInstruction::Constant { value: 42, .. })
        )
    );
}

#[test]
fn lowers_scalar_integer_addition_into_neutral_ir() {
    let source = "verb main() -> Int { return 40 + 2; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar fixture should parse");
    let module =
        lower_scalar_program("actus_root", &program, None).expect("scalar addition should lower");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::IntegerBinary { operation: BackendIntegerBinaryOp::Add, .. }
        )
    ));
}

#[test]
fn lowers_scalar_bitwise_operations_into_neutral_ir() {
    let cases = [
        ("12 & 10", BackendIntegerBinaryOp::BitwiseAnd),
        ("12 | 10", BackendIntegerBinaryOp::BitwiseOr),
        ("12 ^ 10", BackendIntegerBinaryOp::BitwiseXor),
    ];
    for (expression, operation) in cases {
        let source = format!("verb main() -> Int {{ return {expression}; }}");
        let (tokens, errors) = scan(&source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("scalar bitwise fixture should parse");
        let module = lower_scalar_program("actus_root", &program, None)
            .expect("scalar bitwise expression should lower");
        assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(
                instruction,
                BackendInstruction::IntegerBinary { operation: actual, .. } if *actual == operation
            )
        ));
    }
}

#[test]
fn lowers_scalar_division_and_remainder_with_zero_guards() {
    let cases =
        [("20 / 5", BackendIntegerBinaryOp::Divide), ("20 % 6", BackendIntegerBinaryOp::Remainder)];
    for (expression, operation) in cases {
        let source = format!("verb main() -> Int {{ return {expression}; }}");
        let (tokens, errors) = scan(&source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("scalar division fixture should parse");
        let module = lower_scalar_program("actus_root", &program, None)
            .expect("scalar division expression should lower");
        let instructions = module.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .collect::<Vec<_>>();
        let guard_index = instructions
            .iter()
            .position(|instruction| matches!(instruction, BackendInstruction::TrapIfZero { .. }))
            .expect("division must have a zero guard");
        let operation_index = instructions
            .iter()
            .position(|instruction| {
                matches!(
                    instruction,
                    BackendInstruction::IntegerBinary { operation: actual, .. } if *actual == operation
                )
            })
            .expect("division operation must be present");
        assert!(guard_index < operation_index, "zero guard must precede division");
        if operation == BackendIntegerBinaryOp::Divide {
            let overflow_guard_index = instructions
                .iter()
                .position(|instruction| {
                    matches!(
                        instruction,
                        BackendInstruction::TrapIfSignedDivisionOverflow { width: 32, .. }
                    )
                })
                .expect("signed division must have an overflow guard");
            assert!(overflow_guard_index < operation_index);
        }
    }
}

#[test]
fn lowers_scalar_shifts_with_width_guards() {
    let cases = [
        ("1 << 4", BackendIntegerBinaryOp::ShiftLeft),
        ("32 >> 3", BackendIntegerBinaryOp::ShiftRight),
    ];
    for (expression, operation) in cases {
        let source = format!("verb main() -> Int {{ return {expression}; }}");
        let (tokens, errors) = scan(&source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("scalar shift fixture should parse");
        let module = lower_scalar_program("actus_root", &program, None)
            .expect("scalar shift expression should lower");
        let instructions = module.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .collect::<Vec<_>>();
        let guard_index = instructions
            .iter()
            .position(|instruction| {
                matches!(
                    instruction,
                    BackendInstruction::TrapIfShiftCountOutOfRange { width: 32, .. }
                )
            })
            .expect("shift must have a width guard");
        let operation_index = instructions
            .iter()
            .position(|instruction| {
                matches!(
                    instruction,
                    BackendInstruction::IntegerBinary { operation: actual, .. } if *actual == operation
                )
            })
            .expect("shift operation must be present");
        assert!(guard_index < operation_index, "width guard must precede shift");
    }
}

#[test]
fn lowers_scalar_integer_comparison_into_neutral_ir() {
    let source = "verb main() -> Bool { return 1 < 2; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar fixture should parse");
    let module =
        lower_scalar_program("actus_root", &program, None).expect("scalar comparison should lower");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::IntegerCompare {
                operation: BackendIntegerCompareOp::LessThan,
                signed: true,
                ..
            }
        )
    ));
}

#[test]
fn lowers_scalar_integer_unary_into_neutral_ir() {
    let source = "verb main() -> Int { return -7; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar fixture should parse");
    let module =
        lower_scalar_program("actus_root", &program, None).expect("scalar unary should lower");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::IntegerUnary { operation: BackendIntegerUnaryOp::Negate, .. }
        )
    ));
}

#[test]
fn lowers_scalar_owner_declaration_into_a_scoped_value() {
    let source = "verb main() -> Int { erg answer = 40; return answer + 2; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar owner fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("scalar owner declaration should lower");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::IntegerBinary { operation: BackendIntegerBinaryOp::Add, .. }
        )
    ));
}

#[test]
fn lowers_scalar_local_assignment_into_a_new_value() {
    let source = "verb main() -> Int { erg answer = 40; answer = answer + 2; return answer; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar assignment fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("scalar local assignment should lower");
    let mut instructions =
        module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter());
    assert!(instructions.clone().any(|instruction| matches!(
        instruction,
        BackendInstruction::IntegerBinary { operation: BackendIntegerBinaryOp::Add, .. }
    )));
    assert!(instructions.any(|instruction| matches!(
        instruction,
        BackendInstruction::Return { value: Some(_), .. }
    )));
}

#[test]
fn lowers_scalar_compound_assignment_into_integer_binary_ir() {
    let source = "verb main() -> Int { erg answer = 40; answer += 2; return answer; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar compound assignment fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("scalar compound assignment should lower");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::IntegerBinary { operation: BackendIntegerBinaryOp::Add, .. }
        )
    ));
}

#[test]
fn lowers_scalar_compound_bitwise_assignment_into_integer_binary_ir() {
    let source = "verb main() -> Int { erg answer = 8; answer |= 2; return answer; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar compound bitwise fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("scalar compound bitwise assignment should lower");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::IntegerBinary { operation: BackendIntegerBinaryOp::BitwiseOr, .. }
        )
    ));
}

#[test]
fn lowers_scalar_parameter_assignment_in_the_function_scope() {
    let source = "verb main(erg answer: Int) -> Int { answer += 2; return answer; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("scalar parameter assignment fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("scalar parameter assignment should lower");
    assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            BackendInstruction::IntegerBinary { operation: BackendIntegerBinaryOp::Add, .. }
        )
    ));
}

#[test]
fn rejects_outer_scalar_assignment_without_a_branch_merge_contract() {
    let source = "verb main() -> Int { erg answer = 0; if true { answer = 1; }; return answer; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("outer assignment fixture should parse");
    let error = lower_scalar_program("actus_root", &program, None)
        .expect_err("outer assignment without a merge must be rejected");
    let message = error.to_string();
    assert!(message.contains("current lexical scope"), "unexpected error: {message}");
}

#[test]
fn lowers_local_region_cleanup_at_nested_scope_boundary() {
    let source = "verb forward(dat region: Region[u32]) -> Region[u32] { return region; } verb inspect(dat region: Region[u32]) -> Int { { erg local: Region[u32] = forward(region: dat region); } return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("nested Region fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("nested local Region cleanup should lower through the shared pipeline");
    let inspect = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("inspect"))
        .expect("inspect function should be present");
    let nested_block = inspect
        .blocks
        .iter()
        .find(|block| {
            block
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, BackendInstruction::Cleanup { .. }))
        })
        .expect("nested block must contain the local Region cleanup");
    let cleanup_index = nested_block
        .instructions
        .iter()
        .position(|instruction| matches!(instruction, BackendInstruction::Cleanup { .. }))
        .expect("nested local Region must be cleaned at scope exit");
    let return_index = nested_block
        .instructions
        .iter()
        .position(|instruction| matches!(instruction, BackendInstruction::Return { .. }));
    assert!(return_index.is_none() || cleanup_index < return_index.unwrap());
}

#[test]
fn lowers_loop_break_with_region_unwind_cleanup() {
    let source = "verb forward(dat region: Region[u32]) -> Region[u32] { return region; } verb inspect(dat region: Region[u32]) -> Int { loop { erg local: Region[u32] = forward(region: dat region); break; } return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("loop Region fixture should parse");
    let module = lower_scalar_program("actus_root", &program, None)
        .expect("loop Region cleanup should lower through the shared pipeline");
    let inspect = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("inspect"))
        .expect("inspect function should be present");
    let cleanup_count = inspect
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| matches!(instruction, BackendInstruction::Cleanup { .. }))
        .count();
    assert_eq!(cleanup_count, 1, "loop break must clean the Region exactly once");
}

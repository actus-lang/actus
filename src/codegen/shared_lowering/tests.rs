use super::lower_scalar_program;
use crate::codegen::{
    BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
};
use crate::lexer::scan;
use crate::parser::parse;

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

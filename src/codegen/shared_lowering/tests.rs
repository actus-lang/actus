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

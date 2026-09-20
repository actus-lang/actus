use actus::codegen::{NativeInstruction, lower_cleanup_plans};
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

#[test]
fn lowers_semantic_cleanup_plans_without_changing_order() {
    let source = "verb cleanup() { erg first = make(); erg second = make(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let semantic = analyze(&program).expect("source should pass semantic analysis");

    let plans = lower_cleanup_plans(&semantic);
    assert_eq!(
        plans[0].instructions,
        vec![
            NativeInstruction::DropBinding { binding_index: 1 },
            NativeInstruction::DropBinding { binding_index: 0 },
        ]
    );
}

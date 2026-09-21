use actus::codegen::{
    NativeInstruction, lower_cleanup_plans, lower_loop_unwind_plans, lower_return_unwind_plans,
};
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
    assert!(plans[0].span.end > plans[0].span.start);
    assert_eq!(
        plans[0].instructions,
        vec![
            NativeInstruction::DropBinding { binding_index: 1 },
            NativeInstruction::DropBinding { binding_index: 0 },
        ]
    );
}

#[test]
fn lowers_return_and_loop_unwinds_without_changing_scope_order() {
    let source =
        "verb control() { erg outer = make(); loop { erg inner = make(); break; } return; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let semantic = analyze(&program).expect("source should pass semantic analysis");

    let returns = lower_return_unwind_plans(&semantic);
    assert!(returns[0].span.end > returns[0].span.start);
    assert_eq!(returns[0].scopes[0].depth, 1);
    assert_eq!(
        returns[0].scopes[0].instructions,
        vec![NativeInstruction::DropBinding { binding_index: 0 }]
    );

    let loops = lower_loop_unwind_plans(&semantic);
    assert!(loops[0].span.end > loops[0].span.start);
    assert_eq!(loops[0].scopes[0].depth, 2);
    assert_eq!(
        loops[0].scopes[0].instructions,
        vec![NativeInstruction::DropBinding { binding_index: 1 }]
    );
}

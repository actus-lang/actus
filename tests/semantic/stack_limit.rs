use actus::diagnostics::semantic_diagnostic;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze, validate_stack_limit_backend_target};
use actus::target::TargetSpec;

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("stack_limit fixture should parse"))
}

#[test]
fn rejects_direct_stack_limit_recursion_before_codegen() {
    let error = analyze_source(
        "meta stack_limit(bytes: 256u32) verb recurse() -> Int { recurse(); return 0; }",
    )
    .expect_err("stack_limit recursion must be rejected");
    assert!(matches!(error.kind, SemanticErrorKind::StackLimitRecursion));
    assert_eq!(semantic_diagnostic(&error).code(), "E1858");
}

#[test]
fn rejects_mutual_stack_limit_recursion() {
    let error = analyze_source(
        "meta stack_limit(bytes: 256u32) verb first() -> Int { second(); return 0; } meta stack_limit(bytes: 256u32) verb second() -> Int { first(); return 0; }",
    )
    .expect_err("mutual stack_limit recursion must be rejected");
    assert!(matches!(error.kind, SemanticErrorKind::StackLimitRecursion));
}

#[test]
fn leaves_non_stack_limited_recursion_to_existing_contracts() {
    let source =
        "verb first() -> Int { second(); return 0; } verb second() -> Int { first(); return 0; }";
    assert!(analyze_source(source).is_ok());
}

#[test]
fn rejects_reachable_unsafe_extern_without_a_stack_claim() {
    let error = analyze_source(
        "unsafe extern \"C\" verb native() -> Int; meta stack_limit(bytes: 256u32) verb main() -> Int { return native(); }",
    )
    .expect_err("stack_limit must require a native claim");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::MissingStackLimitClaim { ref callee } if callee == "native"
    ));
    assert_eq!(semantic_diagnostic(&error).code(), "E1861");
}

#[test]
fn accepts_reachable_unsafe_extern_with_a_stack_claim() {
    let source = "meta stack_limit(bytes: 128u32) unsafe extern \"C\" verb native() -> Int; meta stack_limit(bytes: 256u32) verb main() -> Int { return native(); }";
    assert!(analyze_source(source).is_ok());
}

#[test]
fn rejects_stack_limit_on_unverified_backend_target_pairs() {
    let source = "meta stack_limit(bytes: 256u32) verb main() -> Int { return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("stack_limit target fixture should parse");
    let target = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
    let error = validate_stack_limit_backend_target(&program, &target, "cranelift")
        .expect_err("Cranelift must not claim stack_limit support");
    assert!(matches!(error.kind, SemanticErrorKind::UnsupportedStackLimitTarget { .. }));
    assert_eq!(semantic_diagnostic(&error).code(), "E1863");
}

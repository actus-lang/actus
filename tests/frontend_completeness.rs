use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::ast::Program;
use actus::diagnostics::{parse_diagnostic, semantic_diagnostic};
use actus::lexer::scan;
use actus::modules::{ModuleError, ModuleResolutionError, ModuleResolver, analyze_with_imports};
use actus::parser::{ParseError, ParseErrorCode, ParseErrorKind, parse};
use actus::semantic::{SemanticError, SemanticErrorKind, analyze};

fn parse_source(source: &str) -> Result<Program, ParseError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens)
}

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticError> {
    analyze(&parse_source(source).expect("semantic fixture should parse"))
}

fn expect_semantic_error(source: &str) -> SemanticError {
    analyze_source(source).expect_err("negative frontend fixture should fail")
}

#[test]
fn parser_keyword_diagnostics_have_stable_fixtures() {
    let unknown_keyword = parse_source("verbb main() { return 0; }").expect_err("unknown keyword");
    assert_eq!(unknown_keyword.code, ParseErrorCode::UnknownKeyword);
    assert!(matches!(
        unknown_keyword.kind,
        ParseErrorKind::UnknownKeyword { ref name } if name == "verbb"
    ));
    assert_eq!(parse_diagnostic(&unknown_keyword).code(), "E0009");

    assert_eq!(parse_diagnostic(&unknown_keyword).code(), "E0009");
}

#[test]
fn must_use_rejects_ignored_results_and_accepts_consumed_results() {
    let ignored = expect_semantic_error(
        "meta must_use(\"check the status\") verb status() -> Int { return 0; } verb main() -> Int { status(); return 0; }",
    );
    assert!(matches!(
        ignored.kind,
        SemanticErrorKind::MustUseIgnored { ref callee, ref reason }
            if callee == "status" && reason == "check the status"
    ));
    assert_eq!(semantic_diagnostic(&ignored).code(), "E1120");

    analyze_source(
        "meta must_use(\"check the status\") verb status() -> Int { return 0; } verb main() -> Int { erg result = status(); return result; }",
    )
    .expect("binding a must-use result should count as consumption");
}

#[test]
fn must_use_tracks_expression_consumption_without_leaking_across_statements() {
    analyze_source(
        "meta must_use(\"check the status\") verb status() -> Int { return 0; } verb main() -> Int { return status() + 1; }",
    )
    .expect("a must-use result consumed by an operator should pass");

    let grouped = expect_semantic_error(
        "meta must_use(\"check the status\") verb status() -> Int { return 0; } verb main() -> Int { (status()); return 0; }",
    );
    assert!(matches!(
        grouped.kind,
        SemanticErrorKind::MustUseIgnored { ref callee, .. } if callee == "status"
    ));

    let nested_statement = expect_semantic_error(
        "meta must_use(\"check the status\") verb status() -> Int { return 0; } verb main() -> Int { if true { status(); } return 0; }",
    );
    assert!(matches!(
        nested_statement.kind,
        SemanticErrorKind::MustUseIgnored { ref callee, .. } if callee == "status"
    ));

    analyze_source(
        "enum Failure { Failed, } meta must_use(\"check the status\") verb status() -> Result[Int, Failure] { return Ok(0); } verb main() -> Result[Int, Failure] { status()?; return Ok(0); }",
    )
    .expect("the try operator should explicitly handle a must-use result");
}

#[test]
fn effects_are_declared_by_callers_and_rejected_when_omitted() {
    let undeclared = expect_semantic_error(
        "meta effect(\"filesystem\") verb load() -> Int { return 0; } verb main() -> Int { load(); return 0; }",
    );
    assert!(matches!(
        undeclared.kind,
        SemanticErrorKind::UndeclaredEffect { ref callee, effect }
            if callee == "load" && effect == actus::ast::Effect::Filesystem
    ));
    assert_eq!(semantic_diagnostic(&undeclared).code(), "E1121");

    analyze_source(
        "meta effect(\"filesystem\") verb load() -> Int { return 0; } meta effect(\"filesystem\") verb main() -> Int { load(); return 0; }",
    )
    .expect("a caller declaring the callee effect should pass");
}

#[test]
fn capabilities_are_declared_by_callers_and_propagate_through_calls() {
    let undeclared = expect_semantic_error(
        "meta capability(\"posix.cloexec\") verb primitive() -> Int { return 0; } verb main() -> Int { primitive(); return 0; }",
    );
    assert!(matches!(
        undeclared.kind,
        SemanticErrorKind::UndeclaredCapability { ref callee, capability }
            if callee == "primitive" && capability == actus::ast::Capability::PosixCloexec
    ));
    assert_eq!(semantic_diagnostic(&undeclared).code(), "E1131");

    analyze_source(
        "meta capability(\"posix.cloexec\") verb primitive() -> Int { return 0; } meta capability(\"posix.cloexec\") verb main() -> Int { primitive(); return 0; }",
    )
    .expect("a caller declaring the callee capability should pass");
}

#[test]
fn create_exclusive_capabilities_are_declared_by_callers() {
    let undeclared = expect_semantic_error(
        "meta capability(\"filesystem.create_exclusive\") verb primitive() -> Int { return 0; } verb main() -> Int { primitive(); return 0; }",
    );
    assert!(matches!(
        undeclared.kind,
        SemanticErrorKind::UndeclaredCapability { ref callee, capability }
            if callee == "primitive" && capability == actus::ast::Capability::FilesystemCreateExclusive
    ));
    assert_eq!(semantic_diagnostic(&undeclared).code(), "E1131");

    analyze_source(
        "meta capability(\"filesystem.create_exclusive\") verb primitive() -> Int { return 0; } meta capability(\"filesystem.create_exclusive\") verb main() -> Int { primitive(); return 0; }",
    )
    .expect("a caller declaring create-exclusive capability should pass");
}

#[test]
fn deterministic_verbs_reject_nondeterministic_effects() {
    for effect in ["filesystem", "network", "entropy", "volatile"] {
        let source = format!(
            "meta effect(\"{effect}\") verb source() -> Int {{ return 0; }} meta deterministic verb stable() -> Int {{ source(); return 0; }}"
        );
        let error = expect_semantic_error(&source);
        assert!(matches!(
            error.kind,
            SemanticErrorKind::NondeterministicCall { ref callee, effect: found }
                if callee == "source" && found.name() == effect
        ));
        assert_eq!(semantic_diagnostic(&error).code(), "E1124");
    }

    analyze_source(
        "meta effect(\"allocation\") verb allocate_resource() -> Int { return 0; } meta effect(\"allocation\") meta deterministic verb stable() -> Int { allocate_resource(); return 0; }",
    )
    .expect("allocation alone does not make a result nondeterministic");
}

#[test]
fn deterministic_verbs_reject_floating_point_at_every_local_boundary() {
    for source in [
        "meta deterministic verb stable() -> f32 { return 1.0f32; }",
        "meta deterministic verb stable() -> Int { erg sample: f64 = 1.0f64; return 0; }",
        "verb source() -> f32 { return 1.0f32; } meta deterministic verb stable() -> Int { source(); return 0; }",
        "unsafe extern \"C\" verb source(erg sample: f32) -> Int; meta deterministic verb stable() -> Int { source(sample: 1.0f32); return 0; }",
    ] {
        let error = expect_semantic_error(source);
        assert!(matches!(error.kind, SemanticErrorKind::DeterministicFloatingPoint));
        assert_eq!(semantic_diagnostic(&error).code(), "E1135");
    }

    analyze_source(
        "verb source() -> f32 { return 1.0f32; } meta deterministic verb stable() -> Int { return 0; }",
    )
    .expect("floating-point declarations outside the deterministic call graph remain valid");
}

#[test]
fn realtime_verbs_reject_effects_unbounded_loops_and_floating_point() {
    for effect in
        ["blocking", "clock", "filesystem", "network", "allocation", "entropy", "volatile"]
    {
        let source = format!(
            "meta effect(\"{effect}\") verb source() -> Int {{ return 0; }} meta realtime verb tick() -> Int {{ source(); return 0; }}"
        );
        let error = expect_semantic_error(&source);
        assert!(matches!(
            error.kind,
            SemanticErrorKind::RealtimeEffect { ref callee, effect: found }
                if callee == "source" && found.name() == effect
        ));
        assert_eq!(semantic_diagnostic(&error).code(), "E1125");
    }

    let loop_error =
        expect_semantic_error("meta realtime verb tick() -> Int { loop { break; } return 0; }");
    assert!(matches!(loop_error.kind, SemanticErrorKind::RealtimeUnboundedLoop));
    assert_eq!(semantic_diagnostic(&loop_error).code(), "E1126");

    let float_error = expect_semantic_error("meta realtime verb tick() -> f32 { return 1.0f32; }");
    assert!(matches!(float_error.kind, SemanticErrorKind::RealtimeFloatingPoint));
    assert_eq!(semantic_diagnostic(&float_error).code(), "E1127");

    analyze_source(
        "meta realtime verb tick() -> Int { for erg index: u32 in 0u32..4u32 { if index == 2u32 { break; } } return 0; }",
    )
    .expect("bounded range loops are valid in realtime code");
}

#[test]
fn realtime_role_contract_is_preserved_by_implementations() {
    let error = expect_semantic_error(
        "role Tick { meta realtime verb tick(abs self: Self) -> Int; } struct Clock { value: Int, } perform Tick for Clock { verb tick(abs self: Clock) -> Int { return self.value; } }",
    );
    assert!(matches!(
        error.kind,
        SemanticErrorKind::RoleMethodRealtimeMismatch { ref method, .. } if method == "tick"
    ));
    assert_eq!(semantic_diagnostic(&error).code(), "E1128");
}

#[test]
fn bounded_contract_proves_ranges_and_call_budgets() {
    analyze_source(
        "meta bounded(max_iterations: 0u32) verb leaf() -> Int { return 0; } meta bounded(max_iterations: 4u32, max_calls: 4u32, max_call_depth: 1u32) verb main() -> Int { for erg index: u32 in 0u32..4u32 { leaf(); } return 0; }",
    )
    .expect("the exact bounded range and call budget should pass");

    let range_error = expect_semantic_error(
        "meta bounded(max_iterations: 3u32) verb main() -> Int { for erg index: u32 in 0u32..4u32 { } return 0; }",
    );
    assert!(matches!(
        range_error.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref dimension, .. } if dimension == "max_iterations"
    ));
    assert_eq!(semantic_diagnostic(&range_error).code(), "E1130");

    let depth_error = expect_semantic_error(
        "meta bounded(max_iterations: 0u32) verb leaf() -> Int { return 0; } meta bounded(max_call_depth: 0u32) verb main() -> Int { leaf(); return 0; }",
    );
    assert!(matches!(
        depth_error.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref dimension, .. } if dimension == "max_call_depth"
    ));

    analyze_source(
        "meta bounded(max_allocations: 1u32, max_allocation_bytes: 4u64) verb main() -> Buffer { return Buffer[4]; }",
    )
    .expect("a fixed buffer allocation should satisfy its resource budget");

    let allocation_error = expect_semantic_error(
        "meta bounded(max_allocations: 0u32) verb main() -> Buffer { return Buffer[4]; }",
    );
    assert!(matches!(
        allocation_error.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref dimension, .. } if dimension == "max_allocations"
    ));

    analyze_source(
        "meta bounded(max_stack_bytes: 4u64) verb main() -> Int { erg value: u32 = 0u32; return value; }",
    )
    .expect("a known scalar local should satisfy its stack budget");

    let missing_callee_bound = expect_semantic_error(
        "verb leaf() -> Int { return 0; } meta bounded(max_calls: 1u32) verb main() -> Int { leaf(); return 0; }",
    );
    assert!(matches!(
        missing_callee_bound.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref required, .. } if required.contains("leaf")
    ));

    analyze_source(
        "meta bounded(max_iterations: 2u32) verb leaf() -> Int { for erg index: u32 in 0u32..2u32 { } return 0; } meta bounded(max_iterations: 2u32) verb main() -> Int { leaf(); return 0; }",
    )
    .expect("a bounded callee contract should propagate to its caller");

    analyze_source(
        "meta bounded(max_iterations: 0u32) unsafe extern \"C\" verb host() -> Int; meta bounded(max_calls: 1u32) verb main() -> Int { host(); return 0; }",
    )
    .expect("a native boundary with an explicit bound should be callable");

    let imported = ModuleFixture::new();
    imported.write("effects/effects.act", "open api;");
    imported.write(
        "effects/api.act",
        "meta bounded(max_iterations: 1u32) open verb leaf() -> Int { return 0; }",
    );
    let imported_source = parse_source(
        "import effects; meta bounded(max_iterations: 1u32) verb main() -> Int { effects::leaf(); return 0; }",
    )
    .expect("imported bounded fixture should parse");
    analyze_with_imports(&imported_source, &ModuleResolver::new(&imported.root))
        .expect("imported bounded signatures must propagate across the facade");

    let recursive_import = ModuleFixture::new();
    recursive_import.write("recursive/recursive.act", "open api;");
    recursive_import.write(
        "recursive/api.act",
        "meta bounded(max_iterations: 1u32) open verb first() -> Int { second(); return 0; } meta bounded(max_iterations: 1u32) open verb second() -> Int { first(); return 0; }",
    );
    let recursive_source =
        parse_source("import recursive; verb main() -> Int { recursive::first(); return 0; }")
            .expect("recursive imported fixture should parse");
    let recursive_error =
        analyze_with_imports(&recursive_source, &ModuleResolver::new(&recursive_import.root))
            .expect_err("recursive imported bodies must be proven before publication");
    let ModuleError::Semantic(recursive_error) = recursive_error else {
        panic!("expected imported semantic error")
    };
    assert!(matches!(
        recursive_error.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref required, .. }
            if required == "recursive call path"
    ));

    analyze_source(
        "meta bounded(max_iterations: 0u32) verb leaf[N: Usize]() -> Int { return 0; } meta bounded(max_iterations: 0u32) verb main() -> Int { leaf[2](); return 0; }",
    )
    .expect("generic specializations must preserve bounded contracts");

    analyze_source(
        "struct File { value: Int, } role Writer { meta bounded(max_iterations: 0u32) verb write(abs self: File) -> Int; } perform Writer for File { meta bounded(max_iterations: 0u32) verb write(abs self: File) -> Int { return self.value; } } meta bounded(max_calls: 1u32, max_call_depth: 1u32) verb send(abs writer: dynamic Writer) -> Int { return writer.write(); }",
    )
    .expect("dynamic method boundaries must preserve bounded role contracts");

    let dynamic_recursive_error = expect_semantic_error(
        "struct File { value: Int, } role Writer { meta bounded(max_iterations: 0u32) verb first(abs self: File) -> Int; meta bounded(max_iterations: 0u32) verb second(abs self: File) -> Int; } perform Writer for File { meta bounded(max_iterations: 0u32) verb first(abs self: File) -> Int { return self.second(); } meta bounded(max_iterations: 0u32) verb second(abs self: File) -> Int { return self.first(); } }",
    );
    assert!(matches!(
        dynamic_recursive_error.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref required, .. }
            if required == "recursive call path"
    ));

    let recursive_error = expect_semantic_error(
        "meta bounded(max_iterations: 1u32) verb first() -> Int { second(); return 0; } meta bounded(max_iterations: 1u32) verb second() -> Int { first(); return 0; }",
    );
    assert!(matches!(
        recursive_error.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref required, .. } if required == "recursive call path"
    ));

    let loop_error = expect_semantic_error(
        "meta bounded(max_iterations: 4u32) verb main() -> Int { loop { break; } return 0; }",
    );
    assert!(matches!(
        loop_error.kind,
        SemanticErrorKind::BoundedBudgetExceeded { ref required, .. } if required == "an unbounded path"
    ));
}

#[test]
fn effects_propagate_through_transitive_and_external_calls() {
    let transitive = expect_semantic_error(
        "meta effect(\"filesystem\") verb leaf() -> Int { return 0; } meta effect(\"filesystem\") verb middle() -> Int { leaf(); return 0; } verb root() -> Int { middle(); return 0; }",
    );
    assert!(matches!(
        transitive.kind,
        SemanticErrorKind::UndeclaredEffect { ref callee, effect }
            if callee == "middle" && effect == actus::ast::Effect::Filesystem
    ));

    let external = expect_semantic_error(
        "meta effect(\"network\") extern \"C\" verb host() -> Int; verb main() -> Int { host(); return 0; }",
    );
    assert!(matches!(
        external.kind,
        SemanticErrorKind::UndeclaredEffect { ref callee, effect }
            if callee == "host" && effect == actus::ast::Effect::Network
    ));

    analyze_source(
        "meta effect(\"network\") extern \"C\" verb host() -> Int; meta effect(\"network\") verb main() -> Int { host(); return 0; }",
    )
    .expect("external effect contracts must be declarable at the caller");
}

#[test]
fn recursive_effect_calls_require_the_declared_contract_on_the_recursive_verb() {
    analyze_source(
        "meta effect(\"filesystem\") verb walk() -> Int { if 1 == 1 { return walk(); } return 0; } meta effect(\"filesystem\") verb main() -> Int { return walk(); }",
    )
    .expect("a recursive call may use the effect declared by its own verb");
}

#[test]
fn imported_facades_preserve_effectful_public_signatures() {
    let fixture = ModuleFixture::new();
    fixture.write("effects/effects.act", "open api;");
    fixture.write(
        "effects/api.act",
        "meta effect(\"filesystem\") open verb load() -> Int { return 0; }",
    );
    let missing = parse_source("import effects; verb main() -> Int { effects::load(); return 0; }")
        .expect("imported effect fixture should parse");
    let error = analyze_with_imports(&missing, &ModuleResolver::new(&fixture.root))
        .expect_err("imported effects must not be erased by a facade");
    let ModuleError::Semantic(error) = error else { panic!("expected semantic error") };
    assert!(matches!(
        error.kind,
        SemanticErrorKind::UndeclaredEffect { ref callee, effect }
            if callee == "effects::load" && effect == actus::ast::Effect::Filesystem
    ));

    let declared = parse_source(
        "import effects; meta effect(\"filesystem\") verb main() -> Int { effects::load(); return 0; }",
    )
    .expect("declared imported effect fixture should parse");
    analyze_with_imports(&declared, &ModuleResolver::new(&fixture.root))
        .expect("declared imported effect should pass");

    let nested = ModuleFixture::new();
    nested.write("teaching/teaching.act", "open transport;");
    nested.write("teaching/transport/transport.act", "open logical;");
    nested.write("teaching/transport/logical/logical.act", "open file;");
    nested.write(
        "teaching/transport/logical/file.act",
        "import teaching; meta effect(\"filesystem\") open verb load() -> Int { return 0; } meta effect(\"filesystem\") verb caller() -> Int { teaching::load(); return 0; }",
    );
    let nested_missing =
        parse_source("import teaching; verb main() -> Int { teaching::load(); return 0; }")
            .expect("nested imported effect fixture should parse");
    let nested_error = analyze_with_imports(&nested_missing, &ModuleResolver::new(&nested.root))
        .expect_err("nested facades must preserve effect contracts");
    let ModuleError::Semantic(nested_error) = nested_error else {
        panic!("expected nested imported semantic error")
    };
    assert!(matches!(
        nested_error.kind,
        SemanticErrorKind::UndeclaredEffect { ref callee, effect }
            if callee == "teaching::load" && effect == actus::ast::Effect::Filesystem
    ));

    let nested_declared = parse_source(
        "import teaching; meta effect(\"filesystem\") verb main() -> Int { teaching::load(); return 0; }",
    )
    .expect("nested declared effect fixture should parse");
    analyze_with_imports(&nested_declared, &ModuleResolver::new(&nested.root))
        .expect("nested declared imported effect should pass");
}

#[test]
fn name_resolution_rules_have_positive_and_negative_fixtures() {
    analyze_source(
        "struct Point { x: Int, } verb known(erg point: Point) { } verb main() -> Int { erg point = Point { x: 1, }; known(point: point); return point.x; }",
    )
    .expect("known types, fields, and verbs should pass");

    let unknown_type = expect_semantic_error("verb main(erg item: Missing) { }");
    assert!(
        matches!(unknown_type.kind, SemanticErrorKind::UnknownType { ref name } if name == "Missing")
    );
    assert_eq!(semantic_diagnostic(&unknown_type).code(), "E1023");

    let unknown_field = expect_semantic_error(
        "struct Point { x: Int, } verb main() -> Int { erg point = Point { x: 1, }; return point.missing; }",
    );
    assert!(matches!(
        unknown_field.kind,
        SemanticErrorKind::UnknownStructField { ref field, .. } if field == "missing"
    ));
    assert_eq!(semantic_diagnostic(&unknown_field).code(), "E1031");

    let unknown_verb = expect_semantic_error("verb main() { missing(); }");
    assert!(
        matches!(unknown_verb.kind, SemanticErrorKind::UnknownVerb { ref name } if name == "missing")
    );
    assert_eq!(semantic_diagnostic(&unknown_verb).code(), "E1069");
}

#[test]
fn duplicate_and_call_resolution_rules_have_positive_and_negative_fixtures() {
    analyze_source("struct Left { value: Int, } struct Right { value: Int, }")
        .expect("distinct declarations should pass");

    let duplicate = expect_semantic_error("struct Point { x: Int, } struct Point { y: Int, }");
    assert!(
        matches!(duplicate.kind, SemanticErrorKind::DuplicateStructName { name } if name == "Point")
    );

    analyze_source(
        "verb copy(erg destination: Buffer, erg source: Buffer) { } verb caller() { erg first = Buffer[1]; erg second = Buffer[1]; copy(destination: first, source: second); }",
    )
    .expect("named same-role arguments should disambiguate a call");
    let ambiguous = expect_semantic_error(
        "verb copy(erg destination: Buffer, erg source: Buffer) { } verb caller() { erg first = Buffer[1]; erg second = Buffer[1]; copy(first, second); }",
    );
    assert!(matches!(
        ambiguous.kind,
        SemanticErrorKind::AmbiguousPositionalCall { callee } if callee == "copy"
    ));
}

#[test]
fn return_and_generic_rules_have_positive_and_negative_fixtures() {
    analyze_source("verb answer() -> Int { return 42; }").expect("complete return path");
    let missing_return = expect_semantic_error("verb answer() -> Int { erg result = 42; }");
    assert!(matches!(missing_return.kind, SemanticErrorKind::MissingReturnValue));

    analyze_source("role Reader { verb read(abs self: Int); } struct Box[T: Reader] { item: T, }")
        .expect("declared generic bound should pass");
    let bad_arity =
        expect_semantic_error("struct Box[T] { item: T, } verb main(erg item: Box[Int, Bool]) { }");
    assert!(
        matches!(bad_arity.kind, SemanticErrorKind::GenericArityMismatch { name, .. } if name == "Box")
    );

    let bad_bound = expect_semantic_error(
        "role Reader { verb read(abs self: Int); } struct Other { value: Int, } struct Box[T: Reader] { item: T, } verb main(erg item: Box[Other]) { }",
    );
    assert!(matches!(bad_bound.kind, SemanticErrorKind::GenericConstraintMismatch { .. }));
}

#[test]
fn conditional_guards_require_bool_values() {
    analyze_source("verb choose(erg ready: Bool) { if ready { print(1); } else { print(0); }; }")
        .expect("Bool conditional guard should pass");

    let invalid =
        expect_semantic_error("verb main() -> Int { if 1 { return 1; } else { return 0; } }");
    assert!(matches!(
        invalid.kind,
        SemanticErrorKind::GuardTypeMismatch { ref found } if found == "Int"
    ));
    assert_eq!(semantic_diagnostic(&invalid).code(), "E1063");
}

#[test]
fn conditional_expression_branches_have_one_value_type() {
    analyze_source("verb choose(erg ready: Bool) -> Int { return if ready { 1 } else { 2 }; }")
        .expect("value-producing branches should unify");

    let mismatch = expect_semantic_error(
        "verb choose(erg ready: Bool) -> Int { return if ready { 1 } else { 2u8 }; }",
    );
    assert!(matches!(
        mismatch.kind,
        SemanticErrorKind::TypeMismatch { ref callee, ref parameter, .. }
            if callee == "if" && parameter == "branches"
    ));
}

#[test]
fn statement_guards_may_omit_else_when_they_do_not_produce_a_value() {
    analyze_source("verb main() { if true { return; } }")
        .expect("statement-form guards should not require an else branch");
}

#[test]
fn value_producing_conditionals_still_require_matching_branches() {
    let mismatch = expect_semantic_error(
        "verb choose(erg ready: Bool) -> Int { erg selected: Int = if ready { 1 }; return selected; }",
    );
    assert!(matches!(
        mismatch.kind,
        SemanticErrorKind::TypeMismatch { ref callee, ref parameter, .. }
            if callee == "if" && parameter == "branches"
    ));
}

#[test]
fn conditional_branches_must_join_ownership_state() {
    let mismatch = expect_semantic_error(
        "verb main() { erg bytes = Buffer[1]; if 1 < 2 { drop(bytes); } else { append(bytes, 1); }; }",
    );
    assert!(
        matches!(mismatch.kind, SemanticErrorKind::BranchStateMismatch { ref name, .. } if name == "bytes")
    );
}

#[test]
fn conditional_and_compound_syntax_fail_with_stable_parser_diagnostics() {
    let missing_branch =
        parse_source("verb main(erg ready: Bool) -> Int { return if ready { 1 } else; }")
            .expect_err("else must be followed by a block or if expression");
    assert!(matches!(
        missing_branch.kind,
        ParseErrorKind::UnexpectedToken { ref expected, ref found }
            if expected == "`{`" && matches!(found, actus::lexer::TokenKind::Semicolon)
    ));
    assert_eq!(parse_diagnostic(&missing_branch).code(), "E0003");

    let incomplete_assignment =
        parse_source("verb main() { erg counter: u32 = 0u32; counter +=; }")
            .expect_err("compound assignment must have a right-hand expression");
    assert!(matches!(
        incomplete_assignment.kind,
        ParseErrorKind::UnexpectedToken { ref expected, ref found }
            if expected == "expression" && matches!(found, actus::lexer::TokenKind::Semicolon)
    ));
    assert_eq!(parse_diagnostic(&incomplete_assignment).code(), "E0003");
}

#[test]
fn malformed_generic_keys_and_primitive_rules_have_fixtures() {
    analyze_source("struct Box[T] { item: T, } verb main(erg item: Box[Int]) { }")
        .expect("well-formed generic key should pass");
    let malformed = SemanticError {
        kind: SemanticErrorKind::MalformedTypeName { name: "Box[".to_owned() },
        span: actus::lexer::SourceSpan::new(0, 4),
    };
    assert_eq!(semantic_diagnostic(&malformed).code(), "E1080");

    analyze_source("verb main(erg bits: u1, erg signed: i8, erg decimal: f32) { }")
        .expect("supported widths and signedness should pass");
    let out_of_range = expect_semantic_error("verb main() { erg bit: u1 = 2; }");
    assert!(matches!(out_of_range.kind, SemanticErrorKind::NumericLiteralOutOfRange { .. }));
}

#[test]
fn module_rules_have_positive_and_negative_fixtures() {
    let fixture = ModuleFixture::new();
    fixture.write("shared/shared.act", "open types;");
    fixture.write("shared/types.act", "open struct Token { byte: Int, }");
    let program = parse_source(
        "import shared; verb main() -> Int { erg token = Token { byte: 7, }; return token.byte; }",
    )
    .expect("valid import should parse");
    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("valid import and facade export should pass");

    let missing = parse_source("import missing; verb main() { return 0; }")
        .expect("missing import fixture should parse");
    let error = analyze_with_imports(&missing, &ModuleResolver::new(&fixture.root))
        .expect_err("unknown import should fail");
    assert!(matches!(error, ModuleError::Resolution(_)));

    fixture.write("driver/gpio/gpio.act", "");
    fixture.write("driver/gpio.act", "");
    let ambiguous = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect_err("ambiguous module roots should fail");
    assert!(matches!(ambiguous, ModuleResolutionError::AmbiguousModule { .. }));
}

#[test]
fn non_codegen_declarations_are_explicitly_accepted() {
    analyze_source(
        "struct Empty { } role Marker { } perform Marker for Empty { } unsafe extern \"C\" verb host(abs input: Buffer) -> Int;",
    )
    .expect("explicit non-codegen declarations must remain semantically valid");
}

struct ModuleFixture {
    root: PathBuf,
}

impl ModuleFixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-frontend-completeness-{stamp}"));
        fs::create_dir_all(&root).expect("create module fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, source: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("module fixture parent")).expect("create parent");
        fs::write(path, source).expect("write module fixture");
    }
}

impl Drop for ModuleFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

use actus::ast::{
    Block, DispatchMode, Expr, Param, ReturnAccess, ReturnType, Role, Stmt, TypeName, VerbDecl,
};
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendBodyProvider, BackendCleanupAction, BackendContractError,
    BackendExternal, BackendFailure, BackendFunction, BackendInstruction, BackendLayout,
    BackendLayoutProvider, BackendModule, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendTargetContext, BackendUnsupportedOperation, BackendValueType,
    NativeInstruction, SymbolIdentity, SymbolKind, lower_semantic_cleanup, lower_verb,
    lower_verb_body, lower_verb_signature, lower_verb_with_cleanup,
};
use actus::lexer::SourceSpan;
use actus::semantic::{
    AccessState, Binding, CleanupAction, OwnershipState, ScopeCleanup, SemanticModel,
};

fn verb_symbol() -> SymbolIdentity {
    SymbolIdentity::new("test", SymbolKind::Verb, "run").expect("valid verb symbol")
}

struct TestLayouts;

struct TestBodyProvider;

impl BackendBodyProvider for TestBodyProvider {
    fn value_for(
        &self,
        _expression: &Expr,
    ) -> Result<actus::codegen::BackendValueId, BackendContractError> {
        Ok(actus::codegen::BackendValueId(7))
    }

    fn callee_for(
        &self,
        callee: &str,
        _span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        SymbolIdentity::new("test", SymbolKind::Verb, callee)
            .map_err(|_| BackendContractError::InvalidCallSymbol)
    }

    fn failure_for(
        &self,
        expression: &Expr,
    ) -> Result<Option<BackendFailure>, BackendContractError> {
        let Expr::Call { callee, .. } = expression else {
            return Ok(None);
        };
        if callee != "Err" {
            return Ok(None);
        }
        Ok(Some(BackendFailure {
            result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Result")
                .expect("valid result symbol"),
            discriminant: 1,
            payload: Some(actus::codegen::BackendValueId(7)),
        }))
    }
}

impl BackendLayoutProvider for TestLayouts {
    fn layout_for(&self, type_name: &TypeName) -> Option<actus::codegen::BackendLayout> {
        (!type_name.name.is_empty())
            .then_some(actus::codegen::BackendLayout { size: 16, alignment: 8 })
    }
}

fn type_name(name: &str, role: Option<Role>) -> TypeName {
    TypeName {
        name: name.to_owned(),
        arguments: Vec::new(),
        reference_role: role,
        span: SourceSpan::new(0, name.len()),
    }
}

fn verb_with_signature() -> VerbDecl {
    VerbDecl {
        is_open: false,
        doc: None,
        contract: None,
        metadata: Vec::new(),
        name: "process".to_owned(),
        generic_parameters: Vec::new(),
        params: vec![
            Param {
                role: Role::Abs,
                name: "view".to_owned(),
                dispatch: DispatchMode::Static,
                ty: type_name("Region", Some(Role::Abs)),
                span: SourceSpan::new(0, 5),
            },
            Param {
                role: Role::Ins,
                name: "loan".to_owned(),
                dispatch: DispatchMode::Static,
                ty: type_name("Region", Some(Role::Ins)),
                span: SourceSpan::new(6, 10),
            },
            Param {
                role: Role::Dat,
                name: "resource".to_owned(),
                dispatch: DispatchMode::Static,
                ty: type_name("Region", Some(Role::Dat)),
                span: SourceSpan::new(11, 19),
            },
            Param {
                role: Role::Erg,
                name: "value".to_owned(),
                dispatch: DispatchMode::Static,
                ty: type_name("u32", Some(Role::Erg)),
                span: SourceSpan::new(20, 25),
            },
        ],
        return_type: Some(ReturnType {
            access: ReturnAccess::Owned,
            ty: type_name("Result", None),
            span: SourceSpan::new(26, 32),
        }),
        body: Block { statements: Vec::new(), span: SourceSpan::new(0, 32) },
        span: SourceSpan::new(0, 32),
    }
}

fn unit_function(parameters: Vec<BackendParameter>, blocks: Vec<BackendBlock>) -> BackendFunction {
    BackendFunction {
        symbol: verb_symbol(),
        parameters,
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks,
        cleanup: Vec::new(),
    }
}

#[test]
fn reports_unsupported_operations_with_target_context() {
    let error = BackendContractError::Unsupported(BackendUnsupportedOperation::new(
        "aggregate-return",
        SourceSpan::new(12, 20),
        BackendTargetContext {
            backend: "llvm".to_owned(),
            target: "thumbv7em-none-eabihf".to_owned(),
        },
        "aggregate return lowering is not implemented",
    ));
    let rendered = error.to_string();
    assert!(rendered.contains("aggregate-return"));
    assert!(rendered.contains("thumbv7em-none-eabihf"));
    assert!(rendered.contains("12"));
}

#[test]
fn validates_aligned_aggregate_layouts() {
    assert!(BackendLayout { size: 16, alignment: 8 }.validate().is_ok());
    assert!(BackendLayout { size: 12, alignment: 8 }.validate().is_err());
    assert!(BackendLayout { size: 16, alignment: 3 }.validate().is_err());
}

#[test]
fn rejects_zero_width_values_and_invalid_parameter_aggregates() {
    let zero_width = unit_function(
        vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Integer { signed: false, width: 0 },
            passing: BackendPassingMode::Direct,
        }],
        vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
    );
    assert_eq!(zero_width.validate(), Err(BackendContractError::InvalidValueType));

    let invalid_aggregate = unit_function(
        vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Aggregate { layout: BackendLayout { size: 12, alignment: 8 } },
            passing: BackendPassingMode::Direct,
        }],
        vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
    );
    assert_eq!(
        invalid_aggregate.validate(),
        Err(BackendContractError::InvalidLayout(
            "aggregate size must be aligned to its declared alignment",
        ))
    );
}

#[test]
fn preserves_role_metadata_without_a_machine_backend() {
    let function = unit_function(
        vec![BackendParameter {
            name: "input".to_owned(),
            role: Role::Abs,
            ty: BackendValueType::Integer { signed: false, width: 32 },
            passing: BackendPassingMode::Borrowed {
                layout: BackendLayout { size: 4, alignment: 4 },
            },
        }],
        vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: None,
                span: SourceSpan::new(0, 1),
            }],
        }],
    );
    assert_eq!(function.parameters[0].role, Role::Abs);
    assert!(function.validate().is_ok());
}

#[test]
fn rejects_role_and_passing_mode_mismatches() {
    let function = unit_function(
        vec![BackendParameter {
            name: "loan".to_owned(),
            role: Role::Ins,
            ty: BackendValueType::Pointer,
            passing: BackendPassingMode::Borrowed {
                layout: BackendLayout { size: 8, alignment: 8 },
            },
        }],
        vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
    );
    assert_eq!(function.validate(), Err(BackendContractError::RolePassingMismatch));
}

#[test]
fn preserves_capability_passing_modes_and_cleanup_actions() {
    let layout = BackendLayout { size: 16, alignment: 8 };
    let function = unit_function(
        vec![
            BackendParameter {
                name: "view".to_owned(),
                role: Role::Abs,
                ty: BackendValueType::Region { layout },
                passing: BackendPassingMode::Borrowed { layout },
            },
            BackendParameter {
                name: "loan".to_owned(),
                role: Role::Ins,
                ty: BackendValueType::Region { layout },
                passing: BackendPassingMode::ExclusiveLoan { layout },
            },
            BackendParameter {
                name: "resource".to_owned(),
                role: Role::Dat,
                ty: BackendValueType::Region { layout },
                passing: BackendPassingMode::OwnershipTransfer { layout },
            },
        ],
        vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Cleanup {
                action: NativeInstruction::ResetArena {
                    binding_index: 2,
                    name: "region".to_owned(),
                },
                span: SourceSpan::new(4, 10),
            }],
        }],
    );
    assert!(function.validate().is_ok());
}

#[test]
fn validates_aggregate_return_slots_without_a_backend() {
    let layout = BackendLayout { size: 16, alignment: 8 };
    let function = BackendFunction {
        symbol: verb_symbol(),
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Aggregate { layout },
            passing: BackendReturnMode::ReturnSlot { layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
        cleanup: Vec::new(),
    };
    assert!(function.validate().is_ok());
}

#[test]
fn rejects_return_slots_for_non_aggregate_values() {
    let function = BackendFunction {
        symbol: verb_symbol(),
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Integer { signed: false, width: 32 },
            passing: BackendReturnMode::ReturnSlot {
                layout: BackendLayout { size: 4, alignment: 4 },
            },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
        cleanup: Vec::new(),
    };
    assert_eq!(function.validate(), Err(BackendContractError::ReturnSlotTypeMismatch));
}

#[test]
fn rejects_duplicate_parameters_and_blocks() {
    let parameter = BackendParameter {
        name: "value".to_owned(),
        role: Role::Erg,
        ty: BackendValueType::Integer { signed: false, width: 32 },
        passing: BackendPassingMode::Direct,
    };
    let function = unit_function(
        vec![parameter.clone(), parameter],
        vec![
            BackendBlock { id: BackendBlockId(0), instructions: Vec::new() },
            BackendBlock { id: BackendBlockId(1), instructions: Vec::new() },
        ],
    );
    assert_eq!(function.validate(), Err(BackendContractError::DuplicateParameterName));

    let mut blocks_only = function;
    blocks_only.parameters.clear();
    blocks_only.blocks[1].id = BackendBlockId(0);
    assert_eq!(blocks_only.validate(), Err(BackendContractError::DuplicateBlockId));
}

#[test]
fn validates_control_flow_targets_without_a_machine_backend() {
    let valid = unit_function(
        Vec::new(),
        vec![
            BackendBlock {
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Jump {
                    target: BackendBlockId(1),
                    span: SourceSpan::new(0, 1),
                }],
            },
            BackendBlock { id: BackendBlockId(1), instructions: Vec::new() },
        ],
    );
    assert!(valid.validate().is_ok());

    let mut invalid = valid;
    invalid.blocks[0].instructions[0] =
        BackendInstruction::Jump { target: BackendBlockId(2), span: SourceSpan::new(0, 1) };
    assert_eq!(invalid.validate(), Err(BackendContractError::InvalidBlockReference));
}

#[test]
fn preserves_typed_failures_without_a_machine_backend() {
    let result_type =
        SymbolIdentity::new("std", SymbolKind::Enum, "Result").expect("valid enum symbol");
    let function = unit_function(
        Vec::new(),
        vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: result_type.clone(),
                    discriminant: 1,
                    payload: None,
                },
                span: SourceSpan::new(8, 18),
            }],
        }],
    );
    assert!(function.validate().is_ok());

    let invalid = unit_function(
        Vec::new(),
        vec![BackendBlock {
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: verb_symbol(),
                    discriminant: 1,
                    payload: None,
                },
                span: SourceSpan::new(8, 18),
            }],
        }],
    );
    assert_eq!(invalid.validate(), Err(BackendContractError::InvalidFailureType));
}

#[test]
fn adapts_validated_semantic_cleanup_to_the_neutral_contract() {
    let mut model = SemanticModel::default();
    model.bindings.push(Binding {
        name: "arena".to_owned(),
        role: Role::Erg,
        ty: Some(actus::ast::BuiltinType::Int),
        span: SourceSpan::new(0, 5),
        ownership: OwnershipState::Active,
        access: AccessState::Mutable,
    });
    model.cleanup_plans.push(ScopeCleanup {
        depth: 1,
        span: SourceSpan::new(0, 5),
        actions: vec![CleanupAction::DropBinding { binding_index: 0 }],
    });

    let plans = lower_semantic_cleanup(&model).expect("valid cleanup should lower");
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].depth, 1);
    assert_eq!(
        plans[0].actions,
        vec![BackendCleanupAction::DropBinding { binding_index: 0, name: "arena".to_owned() }]
    );
}

#[test]
fn rejects_invalid_semantic_cleanup_before_backend_lowering() {
    let mut model = SemanticModel::default();
    model.cleanup_plans.push(ScopeCleanup {
        depth: 1,
        span: SourceSpan::new(0, 1),
        actions: vec![CleanupAction::DropBinding { binding_index: 99 }],
    });

    assert!(matches!(
        lower_semantic_cleanup(&model),
        Err(BackendContractError::InvalidCleanupContract(_))
    ));
}

#[test]
fn lowers_callable_roles_and_layouts_without_a_machine_backend() {
    let signature = lower_verb_signature("app", &verb_with_signature(), &TestLayouts)
        .expect("signature should lower through the neutral provider");
    assert_eq!(signature.symbol.as_str(), "app__verb_process");
    assert!(matches!(signature.parameters[0].passing, BackendPassingMode::Borrowed { .. }));
    assert!(matches!(signature.parameters[1].passing, BackendPassingMode::ExclusiveLoan { .. }));
    assert!(matches!(
        signature.parameters[2].passing,
        BackendPassingMode::OwnershipTransfer { .. }
    ));
    assert!(matches!(signature.return_value.passing, BackendReturnMode::ReturnSlot { .. }));
    assert_eq!(signature.return_value.role, Some(Role::Erg));

    let mut immutable_return = verb_with_signature();
    immutable_return.return_type.as_mut().expect("return type").access = ReturnAccess::Abs;
    let signature = lower_verb_signature("app", &immutable_return, &TestLayouts)
        .expect("immutable return should lower");
    assert_eq!(signature.return_value.role, Some(Role::Abs));
}

#[test]
fn rejects_transferred_scalar_without_a_layout_provider() {
    let mut verb = verb_with_signature();
    verb.params[2].ty = type_name("u32", Some(Role::Dat));
    assert!(matches!(
        lower_verb_signature("app", &verb, &TestLayouts),
        Err(BackendContractError::Unsupported(_))
    ));
}

#[test]
fn validates_functions_and_external_symbols_as_one_neutral_module() {
    let external_symbol =
        SymbolIdentity::new("runtime", SymbolKind::External, "write_byte").expect("valid symbol");
    let module = BackendModule {
        externals: vec![BackendExternal {
            symbol: external_symbol,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
        }],
        functions: vec![unit_function(
            Vec::new(),
            vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
        )],
    };
    assert!(module.validate().is_ok());

    let duplicate = BackendModule {
        externals: Vec::new(),
        functions: vec![
            unit_function(
                Vec::new(),
                vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
            ),
            unit_function(
                Vec::new(),
                vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
            ),
        ],
    };
    assert_eq!(duplicate.validate(), Err(BackendContractError::DuplicateModuleSymbol));
}

#[test]
fn lowers_return_and_direct_calls_through_a_neutral_body_provider() {
    let mut verb = verb_with_signature();
    verb.body = Block {
        statements: vec![
            Stmt::Expression {
                expression: Expr::Call {
                    callee: "emit".to_owned(),
                    arguments: Vec::new(),
                    span: SourceSpan::new(40, 44),
                },
                span: SourceSpan::new(40, 45),
            },
            Stmt::Return { value: None, span: SourceSpan::new(46, 52) },
        ],
        span: SourceSpan::new(40, 52),
    };

    let blocks = lower_verb_body(&verb, &TestBodyProvider).expect("body should lower");
    assert_eq!(blocks.len(), 1);
    assert!(matches!(blocks[0].instructions[0], BackendInstruction::Call { .. }));
    assert!(matches!(blocks[0].instructions[1], BackendInstruction::Return { value: None, .. }));
}

#[test]
fn lowers_if_control_flow_with_explicit_value_provider() {
    let mut verb = verb_with_signature();
    verb.body = Block {
        statements: vec![
            Stmt::If {
                condition: Expr::BoolLiteral { value: true, span: SourceSpan::new(0, 4) },
                then_branch: Block {
                    statements: vec![Stmt::Return { value: None, span: SourceSpan::new(5, 11) }],
                    span: SourceSpan::new(5, 11),
                },
                else_branch: Some(actus::ast::IfBranch::Block(Block {
                    statements: vec![Stmt::Return { value: None, span: SourceSpan::new(12, 18) }],
                    span: SourceSpan::new(12, 18),
                })),
                span: SourceSpan::new(0, 18),
            },
            Stmt::Return { value: None, span: SourceSpan::new(19, 25) },
        ],
        span: SourceSpan::new(0, 25),
    };

    let blocks = lower_verb_body(&verb, &TestBodyProvider).expect("if should lower");
    assert_eq!(blocks.len(), 4);
    assert!(matches!(blocks[0].instructions[0], BackendInstruction::Branch { .. }));
    assert!(matches!(blocks[1].instructions[0], BackendInstruction::Return { .. }));
    assert!(matches!(blocks[2].instructions[0], BackendInstruction::Return { .. }));
    assert!(matches!(blocks[3].instructions[0], BackendInstruction::Return { .. }));
}

#[test]
fn integrates_signature_and_body_into_a_valid_neutral_function() {
    let mut verb = verb_with_signature();
    verb.body = Block {
        statements: vec![Stmt::Return { value: None, span: SourceSpan::new(0, 6) }],
        span: SourceSpan::new(0, 6),
    };

    let function = lower_verb("app", &verb, &TestLayouts, &TestBodyProvider, BackendBlockId(0))
        .expect("signature and body should form a valid function");
    assert_eq!(function.symbol.as_str(), "app__verb_process");
    assert_eq!(function.blocks.len(), 1);
    assert!(function.validate().is_ok());
}

#[test]
fn preserves_cleanup_plans_on_the_neutral_function_contract() {
    let mut verb = verb_with_signature();
    verb.body = Block {
        statements: vec![Stmt::Return { value: None, span: SourceSpan::new(0, 6) }],
        span: SourceSpan::new(0, 6),
    };
    let cleanup = vec![actus::codegen::BackendCleanupPlan {
        depth: 2,
        span: SourceSpan::new(0, 6),
        actions: vec![BackendCleanupAction::DropBinding {
            binding_index: 0,
            name: "resource".to_owned(),
        }],
    }];

    let function = lower_verb_with_cleanup(
        "app",
        &verb,
        &TestLayouts,
        &TestBodyProvider,
        BackendBlockId(0),
        cleanup.clone(),
    )
    .expect("cleanup should remain part of the neutral function");
    assert_eq!(function.cleanup, cleanup);
    assert!(function.validate().is_ok());
}

#[test]
fn lowers_typed_failures_without_backend_specific_handles() {
    let mut verb = verb_with_signature();
    verb.body = Block {
        statements: vec![Stmt::Return {
            value: Some(Expr::Call {
                callee: "Err".to_owned(),
                arguments: Vec::new(),
                span: SourceSpan::new(0, 6),
            }),
            span: SourceSpan::new(0, 10),
        }],
        span: SourceSpan::new(0, 10),
    };

    let function = lower_verb("app", &verb, &TestLayouts, &TestBodyProvider, BackendBlockId(0))
        .expect("typed failure should lower through the neutral contract");
    let BackendInstruction::Fail { failure, .. } = &function.blocks[0].instructions[0] else {
        panic!("expected a typed failure instruction");
    };
    assert_eq!(failure.discriminant, 1);
    assert!(function.validate().is_ok());
}

#[test]
fn rejects_body_constructs_without_a_neutral_lowering_contract() {
    let mut verb = verb_with_signature();
    verb.body = Block {
        statements: vec![Stmt::Loop(Block { statements: Vec::new(), span: SourceSpan::new(0, 4) })],
        span: SourceSpan::new(0, 4),
    };

    let error = lower_verb_body(&verb, &TestBodyProvider).expect_err("loop is not supported yet");
    assert!(matches!(error, BackendContractError::Unsupported(_)));
    assert!(error.to_string().contains("loop"));
}

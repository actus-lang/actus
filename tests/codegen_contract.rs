use actus::ast::Role;
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendContractError, BackendFailure, BackendFunction,
    BackendInstruction, BackendLayout, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendTargetContext, BackendUnsupportedOperation, BackendValueType,
    NativeInstruction, SymbolIdentity, SymbolKind,
};
use actus::lexer::SourceSpan;

fn verb_symbol() -> SymbolIdentity {
    SymbolIdentity::new("test", SymbolKind::Verb, "run").expect("valid verb symbol")
}

fn unit_function(parameters: Vec<BackendParameter>, blocks: Vec<BackendBlock>) -> BackendFunction {
    BackendFunction {
        symbol: verb_symbol(),
        parameters,
        return_value: BackendReturn {
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks,
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
                ty: BackendValueType::Aggregate { layout },
                passing: BackendPassingMode::Borrowed { layout },
            },
            BackendParameter {
                name: "loan".to_owned(),
                role: Role::Ins,
                ty: BackendValueType::Aggregate { layout },
                passing: BackendPassingMode::ExclusiveLoan { layout },
            },
            BackendParameter {
                name: "resource".to_owned(),
                role: Role::Dat,
                ty: BackendValueType::Aggregate { layout },
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
            ty: BackendValueType::Aggregate { layout },
            passing: BackendReturnMode::ReturnSlot { layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
    };
    assert!(function.validate().is_ok());
}

#[test]
fn rejects_return_slots_for_non_aggregate_values() {
    let function = BackendFunction {
        symbol: verb_symbol(),
        parameters: Vec::new(),
        return_value: BackendReturn {
            ty: BackendValueType::Integer { signed: false, width: 32 },
            passing: BackendReturnMode::ReturnSlot {
                layout: BackendLayout { size: 4, alignment: 4 },
            },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
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

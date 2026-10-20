use super::super::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendContractError, BackendEdgeArgument,
    BackendExpressionProvider, BackendInstruction, BackendValueId, BodyLowering,
};
use super::context::CaseBranchInput;
use crate::ast::Expr;
use crate::lexer::SourceSpan;

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(crate) fn lower_case_statement_with_carried(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        continuation_id: BackendBlockId,
        carried: &[super::super::loops::CarriedBinding],
        instructions: Vec<BackendInstruction>,
    ) -> Result<(BackendBlock, Vec<BackendBlockParameter>), BackendContractError> {
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(super::super::super::body_contract::unsupported(
                span,
                "enum case expression expected",
            ));
        };
        let continuation_values = carried
            .iter()
            .map(|binding| self.provider.allocate_value(binding.ty))
            .collect::<Vec<_>>();
        let continuation_nominal_types = carried
            .iter()
            .map(|binding| self.provider.nominal_type(binding.value))
            .collect::<Vec<_>>();
        let continuation_parameters = carried
            .iter()
            .zip(&continuation_values)
            .map(|(binding, value)| BackendBlockParameter { value: *value, ty: binding.ty, span })
            .collect::<Vec<_>>();
        let initial_values =
            carried.iter().map(|binding| (binding.name.clone(), binding.value)).collect();
        let entry_instructions = lower_carried_case_branches(
            self,
            CarriedCaseInput {
                id,
                mode: *mode,
                subject,
                branches,
                branch_count: branches.len(),
                span,
                continuation_id,
                carried,
                initial_values,
                instructions,
            },
        )?;
        for (binding, value) in carried.iter().zip(&continuation_values) {
            self.provider.set_binding_value(&binding.name, *value);
        }
        self.remember_case_nominal_types(&continuation_values, continuation_nominal_types);
        Ok((
            BackendBlock { parameters: Vec::new(), id, instructions: entry_instructions },
            continuation_parameters,
        ))
    }

    fn remember_case_nominal_types(
        &mut self,
        values: &[BackendValueId],
        nominal_types: Vec<Option<crate::ast::TypeName>>,
    ) {
        for (value, type_name) in values.iter().zip(nominal_types) {
            if let Some(type_name) = type_name {
                self.provider.remember_nominal_type(*value, type_name);
            }
        }
    }
}

struct CarriedCaseInput<'a> {
    id: BackendBlockId,
    mode: crate::ast::CaseMode,
    subject: &'a Expr,
    branches: &'a [crate::ast::CaseBranch],
    branch_count: usize,
    span: SourceSpan,
    continuation_id: BackendBlockId,
    carried: &'a [super::super::loops::CarriedBinding],
    initial_values: std::collections::BTreeMap<String, BackendValueId>,
    instructions: Vec<BackendInstruction>,
}

fn lower_carried_case_branches<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    input: CarriedCaseInput<'_>,
) -> Result<Vec<BackendInstruction>, BackendContractError> {
    let CarriedCaseInput {
        id,
        mode,
        subject,
        branches,
        branch_count,
        span,
        continuation_id,
        carried,
        initial_values,
        mut instructions,
    } = input;
    let mut current_id = id;
    let mut entry_instructions = None;
    for (index, branch) in branches.iter().enumerate() {
        let (following_id, else_block) = lower_carried_case_branch(
            lowering,
            CarriedBranchInput {
                index,
                branch,
                branch_count,
                mode,
                subject,
                span,
                continuation_id,
                carried,
                initial_values: &initial_values,
            },
            &mut instructions,
        )?;
        if let Some(following_id) = following_id {
            if let Some(entry_instructions) = entry_instructions.take() {
                lowering.blocks.push(BackendBlock {
                    parameters: Vec::new(),
                    id: current_id,
                    instructions: entry_instructions,
                });
            } else {
                entry_instructions = Some(std::mem::take(&mut instructions));
            }
            current_id = following_id;
        } else {
            return Ok(finish_carried_case(
                lowering,
                current_id,
                else_block,
                span,
                instructions,
                entry_instructions,
            ));
        }
    }
    Err(super::super::super::body_contract::unsupported(span, "empty enum case"))
}

fn finish_carried_case<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    current_id: BackendBlockId,
    else_block: BackendBlockId,
    span: SourceSpan,
    instructions: Vec<BackendInstruction>,
    entry_instructions: Option<Vec<BackendInstruction>>,
) -> Vec<BackendInstruction> {
    lowering.blocks.push(BackendBlock {
        parameters: Vec::new(),
        id: else_block,
        instructions: vec![BackendInstruction::Trap {
            reason: "non-exhaustive enum case".to_owned(),
            span,
        }],
    });
    if let Some(entry) = entry_instructions {
        lowering.blocks.push(BackendBlock { parameters: Vec::new(), id: current_id, instructions });
        entry
    } else {
        instructions
    }
}

struct CarriedBranchInput<'a> {
    index: usize,
    branch: &'a crate::ast::CaseBranch,
    branch_count: usize,
    mode: crate::ast::CaseMode,
    subject: &'a Expr,
    span: SourceSpan,
    continuation_id: BackendBlockId,
    carried: &'a [super::super::loops::CarriedBinding],
    initial_values: &'a std::collections::BTreeMap<String, BackendValueId>,
}

fn lower_carried_case_branch<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    input: CarriedBranchInput<'_>,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(Option<BackendBlockId>, BackendBlockId), BackendContractError> {
    for (name, value) in input.initial_values {
        lowering.provider.set_binding_value(name, *value);
    }
    let (is_last, blocks) = lowering.prepare_case_branch(input.index, input.branch_count)?;
    let branch_start = lowering.blocks.len();
    let (following_id, else_block) = super::lower_case_branch(
        lowering,
        CaseBranchInput {
            subject: input.subject,
            branch: input.branch,
            is_last,
            blocks,
            mode: input.mode,
            span: input.span,
            fallthrough: Some(input.continuation_id),
            preserve_loop_bindings: true,
        },
        instructions,
    )?;
    let arguments = input
        .carried
        .iter()
        .map(|binding| BackendEdgeArgument {
            value: carried_branch_value(lowering, input.branch, binding),
            ty: binding.ty,
            span: input.span,
        })
        .collect::<Vec<_>>();
    for block in lowering.blocks.iter_mut().skip(branch_start) {
        super::super::loops::replace_jump_arguments_in_block(
            block,
            input.continuation_id,
            &arguments,
        );
    }
    Ok((following_id, else_block))
}

fn carried_branch_value<P: BackendExpressionProvider>(
    lowering: &BodyLowering<'_, P>,
    _branch: &crate::ast::CaseBranch,
    binding: &super::super::loops::CarriedBinding,
) -> BackendValueId {
    lowering.provider.binding_value(&binding.name).unwrap_or(binding.value)
}

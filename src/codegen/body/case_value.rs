use std::collections::BTreeMap;

use super::super::BackendBlockParameter;
use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendEdgeArgument,
    BackendExpressionProvider, BackendInstruction, BackendValueId, BodyLowering,
};
use crate::ast::{Block, CaseBody, Expr, Pattern, Stmt};
use crate::codegen::BackendValueType;
use crate::lexer::SourceSpan;

use super::loop_support::collect_assignment_names;
use super::loops::CarriedBinding;

struct CaseBranchValue {
    block: BackendBlockId,
    value: BackendValueId,
    carried_values: Vec<BackendValueId>,
}

enum CaseBlockValue {
    Value(BackendValueId),
    Diverged,
}

struct CaseOwnerBranches {
    merge_id: BackendBlockId,
    branch_values: Vec<CaseBranchValue>,
    current_id: BackendBlockId,
    entry_instructions: Option<Vec<BackendInstruction>>,
    instructions: Vec<BackendInstruction>,
}

pub(super) struct CaseOwnerInput<'a> {
    pub(super) id: BackendBlockId,
    pub(super) block: &'a Block,
    pub(super) index: usize,
    pub(super) name: &'a str,
    pub(super) declared_type: Option<&'a str>,
    pub(super) expression: &'a Expr,
    pub(super) span: SourceSpan,
    pub(super) fallthrough: Option<BackendBlockId>,
    pub(super) instructions: Vec<BackendInstruction>,
}

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_case_owner(
        &mut self,
        input: CaseOwnerInput<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        let CaseOwnerInput {
            id,
            block,
            index,
            name,
            declared_type,
            expression,
            span,
            fallthrough,
            instructions,
        } = input;
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "enum case value expected"));
        };
        let carried = self.collect_case_carried_bindings(branches);
        let initial_values = carried
            .iter()
            .map(|binding| (binding.name.clone(), binding.value))
            .collect::<BTreeMap<_, _>>();
        let CaseOwnerBranches {
            merge_id,
            branch_values,
            current_id,
            entry_instructions,
            mut instructions,
        } = self.lower_case_owner_branches(
            id,
            *mode,
            subject,
            branches,
            span,
            &carried,
            &initial_values,
            instructions,
        )?;
        let result_type = self.case_result_type(&branch_values, span)?;
        let continuation_block = self.lower_case_owner_continuation(
            name,
            declared_type,
            block,
            index,
            fallthrough,
            merge_id,
            result_type,
            &carried,
            span,
        )?;
        self.blocks.push(continuation_block);
        self.attach_case_value_branches(branch_values, merge_id, result_type, &carried, span)?;
        if let Some(entry_instructions) = entry_instructions {
            self.blocks.push(BackendBlock { parameters: Vec::new(), id: current_id, instructions });
            instructions = entry_instructions;
        }
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn case_result_type(
        &self,
        branch_values: &[CaseBranchValue],
        span: SourceSpan,
    ) -> Result<BackendValueType, BackendContractError> {
        let Some(first_value) = branch_values.first().map(|branch| branch.value) else {
            return Err(unsupported(span, "empty enum case"));
        };
        self.provider
            .value_type(first_value)
            .ok_or_else(|| unsupported(span, "case result type is unavailable"))
    }

    fn lower_case_owner_continuation(
        &mut self,
        name: &str,
        declared_type: Option<&str>,
        block: &Block,
        index: usize,
        fallthrough: Option<BackendBlockId>,
        merge_id: BackendBlockId,
        result_type: BackendValueType,
        carried: &[CarriedBinding],
        span: SourceSpan,
    ) -> Result<BackendBlock, BackendContractError> {
        let result = self.provider.allocate_value(result_type);
        let carried_values = carried
            .iter()
            .map(|binding| self.provider.allocate_value(binding.ty))
            .collect::<Vec<_>>();
        for (binding, value) in carried.iter().zip(&carried_values) {
            self.provider.set_binding_value(&binding.name, *value);
            if let Some(type_name) = &binding.nominal_type {
                self.provider.remember_nominal_type(*value, type_name.clone());
            }
        }
        let mut merge_instructions = Vec::new();
        self.provider.bind_owner(name, result, declared_type, span, &mut merge_instructions)?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(merge_id, &continuation, fallthrough)?;
        let mut parameters = vec![BackendBlockParameter { value: result, ty: result_type, span }];
        parameters.extend(
            carried_values.iter().zip(carried).map(|(value, binding)| BackendBlockParameter {
                value: *value,
                ty: binding.ty,
                span,
            }),
        );
        continuation_block.parameters = parameters;
        continuation_block.instructions.splice(0..0, merge_instructions);
        Ok(continuation_block)
    }

    fn attach_case_value_branches(
        &mut self,
        branch_values: Vec<CaseBranchValue>,
        merge_id: BackendBlockId,
        result_type: BackendValueType,
        carried: &[CarriedBinding],
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        for branch in branch_values {
            let branch_id = branch.block;
            let value = branch.value;
            let value_type = self
                .provider
                .value_type(value)
                .ok_or_else(|| unsupported(span, "case branch type is unavailable"))?;
            if value_type != result_type {
                return Err(unsupported(span, "case branches have different value types"));
            }
            if branch.carried_values.len() != carried.len() {
                return Err(unsupported(span, "case carried values have inconsistent arity"));
            }
            let mut arguments = vec![BackendEdgeArgument { value, ty: result_type, span }];
            for (binding, carried_value) in carried.iter().zip(branch.carried_values) {
                let carried_type = self
                    .provider
                    .value_type(carried_value)
                    .ok_or_else(|| unsupported(span, "case carried value type is unavailable"))?;
                if carried_type != binding.ty {
                    return Err(unsupported(span, "case carried values have different types"));
                }
                arguments.push(BackendEdgeArgument { value: carried_value, ty: binding.ty, span });
            }
            if let Some(branch) = self.blocks.iter_mut().find(|candidate| candidate.id == branch_id)
            {
                branch.instructions.push(BackendInstruction::Jump {
                    target: merge_id,
                    arguments,
                    span,
                });
            }
        }
        Ok(())
    }

    fn lower_case_owner_branches(
        &mut self,
        id: BackendBlockId,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[crate::ast::CaseBranch],
        span: SourceSpan,
        carried: &[CarriedBinding],
        initial_values: &BTreeMap<String, BackendValueId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<CaseOwnerBranches, BackendContractError> {
        let merge_id = self.allocate_block();
        let mut branch_values = Vec::new();
        let mut current_id = id;
        let mut entry_instructions = None;
        for (index, branch) in branches.iter().enumerate() {
            for (name, value) in initial_values {
                self.provider.set_binding_value(name, *value);
            }
            let (matched, branch_value, following_id, else_id) = self.lower_case_owner_branch(
                subject,
                branch,
                index,
                branches.len(),
                mode,
                span,
                &mut instructions,
            )?;
            if let Some((value_block, value)) = branch_value {
                let carried_values = carried
                    .iter()
                    .map(|binding| {
                        self.provider.binding_value(&binding.name).unwrap_or(binding.value)
                    })
                    .collect::<Vec<_>>();
                branch_values.push(CaseBranchValue { block: value_block, value, carried_values });
            }
            self.blocks.push(matched);
            self.advance_case_owner_branch(
                following_id,
                &mut current_id,
                &mut entry_instructions,
                &mut instructions,
                else_id,
                span,
            );
        }
        Ok(CaseOwnerBranches {
            merge_id,
            branch_values,
            current_id,
            entry_instructions,
            instructions,
        })
    }

    fn lower_case_owner_branch(
        &mut self,
        subject: &Expr,
        branch: &crate::ast::CaseBranch,
        index: usize,
        branch_count: usize,
        mode: crate::ast::CaseMode,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<
        (
            BackendBlock,
            Option<(BackendBlockId, BackendValueId)>,
            Option<BackendBlockId>,
            BackendBlockId,
        ),
        BackendContractError,
    > {
        let matched_id = self.allocate_block();
        let is_last = index + 1 == branch_count;
        let following_id = (!is_last).then(|| self.allocate_block());
        let (subject_value, condition) =
            super::case::lower_case_condition(self, subject, branch, is_last, instructions)?;
        let else_id = following_id.unwrap_or_else(|| self.allocate_block());
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: matched_id,
            else_block: else_id,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span: branch.span,
        });
        let (matched, branch_value) =
            self.lower_case_value_branch(matched_id, branch, mode, subject_value, span)?;
        Ok((matched, branch_value, following_id, else_id))
    }

    fn advance_case_owner_branch(
        &mut self,
        following_id: Option<BackendBlockId>,
        current_id: &mut BackendBlockId,
        entry_instructions: &mut Option<Vec<BackendInstruction>>,
        instructions: &mut Vec<BackendInstruction>,
        else_id: BackendBlockId,
        span: SourceSpan,
    ) {
        if let Some(following_id) = following_id {
            if entry_instructions.is_none() {
                *entry_instructions = Some(std::mem::take(instructions));
            } else {
                self.blocks.push(BackendBlock {
                    parameters: Vec::new(),
                    id: *current_id,
                    instructions: std::mem::take(instructions),
                });
            }
            *current_id = following_id;
        } else {
            self.blocks.push(BackendBlock {
                parameters: Vec::new(),
                id: else_id,
                instructions: vec![BackendInstruction::Trap {
                    reason: "non-exhaustive enum case".to_owned(),
                    span,
                }],
            });
        }
    }

    fn lower_case_value_branch(
        &mut self,
        id: BackendBlockId,
        branch: &crate::ast::CaseBranch,
        mode: crate::ast::CaseMode,
        subject: super::BackendValueId,
        span: SourceSpan,
    ) -> Result<(BackendBlock, Option<(BackendBlockId, super::BackendValueId)>), BackendContractError>
    {
        self.provider.enter_scope(branch.span);
        let mut instructions = Vec::new();
        if let Pattern::Variant { variant, payload, .. } = &branch.pattern {
            self.provider.bind_enum_variant_payload(
                subject,
                variant,
                Some(payload),
                mode,
                branch.span,
                &mut instructions,
            )?;
        }
        let value = match &branch.body {
            CaseBody::Expression(expression)
                if matches!(expression.as_ref(), Expr::Case { .. }) =>
            {
                let (entry, merge_id, value) =
                    self.lower_nested_case_value(id, expression, span, instructions)?;
                return Ok((entry, Some((merge_id, value))));
            }
            CaseBody::Expression(expression) => {
                self.provider.lower_expression(expression, &mut instructions)?
            }
            CaseBody::Block(block) => {
                match self.lower_case_value_block(branch, block, span, &mut instructions)? {
                    CaseBlockValue::Value(value) => value,
                    CaseBlockValue::Diverged => {
                        return Ok((
                            BackendBlock { parameters: Vec::new(), id, instructions },
                            None,
                        ));
                    }
                }
            }
        };
        self.provider.exit_scope(branch.span, &mut instructions)?;
        self.provider.value_type(value).ok_or_else(|| unsupported(span, "case branch type"))?;
        Ok((BackendBlock { parameters: Vec::new(), id, instructions }, Some((id, value))))
    }

    fn lower_case_value_block(
        &mut self,
        branch: &crate::ast::CaseBranch,
        block: &Block,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<CaseBlockValue, BackendContractError> {
        let Some(last) = block.statements.last() else {
            return Err(unsupported(span, "case value block is empty"));
        };
        if matches!(last, Stmt::Return { .. }) {
            for statement in &block.statements {
                self.lower_statement(statement, instructions)?;
            }
            self.provider.exit_scope(branch.span, instructions)?;
            return Ok(CaseBlockValue::Diverged);
        }
        for statement in &block.statements[..block.statements.len() - 1] {
            self.lower_statement(statement, instructions)?;
        }
        let expression = match last {
            Stmt::Expression { expression, .. } => expression.clone(),
            Stmt::If { condition, then_branch, else_branch, span: if_span } => Expr::If {
                condition: Box::new(condition.clone()),
                then_branch: then_branch.clone(),
                else_branch: else_branch.clone(),
                span: *if_span,
            },
            _ => return Err(unsupported(span, "case value block requires a tail expression")),
        };
        Ok(CaseBlockValue::Value(self.provider.lower_expression(&expression, instructions)?))
    }

    fn lower_nested_case_value(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        instructions: Vec<BackendInstruction>,
    ) -> Result<(BackendBlock, BackendBlockId, BackendValueId), BackendContractError> {
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "nested enum case expected"));
        };
        let empty_carried: [CarriedBinding; 0] = [];
        let empty_initial_values: BTreeMap<String, BackendValueId> = BTreeMap::new();
        let lowered = self.lower_case_owner_branches(
            id,
            *mode,
            subject,
            branches,
            span,
            &empty_carried,
            &empty_initial_values,
            instructions,
        )?;
        let Some(first_value) = lowered.branch_values.first().map(|branch| branch.value) else {
            return Err(unsupported(span, "empty nested enum case"));
        };
        let result_type = self
            .provider
            .value_type(first_value)
            .ok_or_else(|| unsupported(span, "nested case result type is unavailable"))?;
        let result = self.provider.allocate_value(result_type);
        self.blocks.push(BackendBlock {
            parameters: vec![BackendBlockParameter { value: result, ty: result_type, span }],
            id: lowered.merge_id,
            instructions: Vec::new(),
        });
        self.attach_case_value_branches(
            lowered.branch_values,
            lowered.merge_id,
            result_type,
            &empty_carried,
            span,
        )?;
        let entry = if let Some(entry_instructions) = lowered.entry_instructions {
            self.blocks.push(BackendBlock {
                parameters: Vec::new(),
                id: lowered.current_id,
                instructions: lowered.instructions,
            });
            BackendBlock { parameters: Vec::new(), id, instructions: entry_instructions }
        } else {
            BackendBlock { parameters: Vec::new(), id, instructions: lowered.instructions }
        };
        Ok((entry, lowered.merge_id, result))
    }
}

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    fn collect_case_carried_bindings(
        &self,
        branches: &[crate::ast::CaseBranch],
    ) -> Vec<CarriedBinding> {
        let mut names = Vec::new();
        for branch in branches {
            if let CaseBody::Block(block) = &branch.body {
                collect_assignment_names(block, &mut names);
            }
        }
        names.sort();
        names.dedup();
        names
            .into_iter()
            .filter_map(|name| {
                let value = self.provider.binding_value(&name)?;
                let ty = self.provider.value_type(value)?;
                Some(CarriedBinding {
                    name,
                    value,
                    ty,
                    nominal_type: self.provider.nominal_type(value),
                })
            })
            .collect::<Vec<_>>()
    }
}

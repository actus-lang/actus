use super::body_contract::{statement_span, terminates, unsupported, unsupported_name};
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendEdgeArgument, BackendFailure,
    BackendInstruction, BackendIntegerBinaryOp, BackendValueId, SymbolIdentity,
};
use crate::ast::{Block, CompoundAssignmentOp, Expr, IfBranch, Place, Stmt, VerbDecl};
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;
mod case;
mod loops;
mod provider;
pub use provider::{BackendBodyProvider, BackendExpressionProvider};

pub fn lower_verb_body(
    verb: &VerbDecl,
    provider: &impl BackendBodyProvider,
) -> Result<Vec<BackendBlock>, BackendContractError> {
    let mut legacy = LegacyExpressionProvider { provider };
    lower_verb_body_with_expressions(verb, &mut legacy)
}

pub fn lower_verb_body_with_expressions(
    verb: &VerbDecl,
    provider: &mut impl BackendExpressionProvider,
) -> Result<Vec<BackendBlock>, BackendContractError> {
    let mut lowering =
        BodyLowering { provider, next_block: 1, blocks: Vec::new(), loop_targets: Vec::new() };
    let entry = BackendBlockId(0);
    let entry_block = lowering.lower_block(entry, &verb.body, None)?;
    lowering.blocks.insert(0, entry_block);
    Ok(lowering.blocks)
}

pub(super) struct BodyLowering<'a, P> {
    pub(super) provider: &'a mut P,
    next_block: u32,
    pub(super) blocks: Vec<BackendBlock>,
    loop_targets: Vec<LoopTargets>,
}

#[derive(Clone)]
struct LoopTargets {
    continue_target: BackendBlockId,
    break_target: BackendBlockId,
    continue_values: Vec<String>,
    break_values: Vec<String>,
}

struct LegacyExpressionProvider<'a, P> {
    provider: &'a P,
}

impl<P: BackendBodyProvider> BackendExpressionProvider for LegacyExpressionProvider<'_, P> {
    fn lower_expression(
        &mut self,
        expression: &Expr,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        self.provider.value_for(expression)
    }

    fn failure_for(
        &mut self,
        expression: &Expr,
    ) -> Result<Option<BackendFailure>, BackendContractError> {
        self.provider.failure_for(expression)
    }

    fn callee_for(
        &mut self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        self.provider.callee_for(callee, span)
    }
}

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_block(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        fallthrough: Option<BackendBlockId>,
    ) -> Result<BackendBlock, BackendContractError> {
        self.provider.enter_scope(block.span);
        let result = self.lower_block_in_scope(id, block, fallthrough);
        match result {
            Ok(mut lowered) => {
                self.provider.exit_scope(block.span, &mut lowered.instructions)?;
                Ok(lowered)
            }
            Err(error) => {
                let mut discarded = Vec::new();
                let _ = self.provider.exit_scope(block.span, &mut discarded);
                Err(error)
            }
        }
    }

    fn lower_block_in_scope(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        fallthrough: Option<BackendBlockId>,
    ) -> Result<BackendBlock, BackendContractError> {
        let mut instructions = Vec::new();
        for (index, statement) in block.statements.iter().enumerate() {
            if matches!(
                statement,
                Stmt::Loop(_)
                    | Stmt::ForRange { .. }
                    | Stmt::ForArray { .. }
                    | Stmt::Block(_)
                    | Stmt::If { .. }
                    | Stmt::Return { value: Some(Expr::Case { .. }), .. }
            ) {
                if let Some(result) = self.lower_structured_statement(
                    id,
                    block,
                    index,
                    statement,
                    fallthrough,
                    std::mem::take(&mut instructions),
                )? {
                    return Ok(result);
                }
            }
            self.lower_statement(statement, &mut instructions)?;
            if terminates(instructions.last()) {
                break;
            }
        }
        Ok(self.finish_block(id, block.span, fallthrough, instructions))
    }

    fn lower_structured_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<Option<BackendBlock>, BackendContractError> {
        match statement {
            Stmt::Loop(nested) => self
                .lower_loop_statement(id, block, index, nested, fallthrough, instructions)
                .map(Some),
            Stmt::ForRange { .. } | Stmt::ForArray { .. } => self
                .lower_for_statement(id, block, index, statement, fallthrough, instructions)
                .map(Some),
            Stmt::Block(nested) => self
                .lower_nested_block_statement(id, block, index, nested, fallthrough, instructions)
                .map(Some),
            Stmt::If { condition, then_branch, else_branch, span } => {
                let continuation_id = self.allocate_block();
                let mut instructions = instructions;
                let branches_terminate = self.lower_if(
                    condition,
                    then_branch,
                    else_branch.as_ref(),
                    *span,
                    continuation_id,
                    &mut instructions,
                )?;
                if branches_terminate {
                    return Ok(Some(BackendBlock { parameters: Vec::new(), id, instructions }));
                }
                let continuation =
                    Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
                let continuation_block =
                    self.lower_block(continuation_id, &continuation, fallthrough)?;
                self.blocks.push(continuation_block);
                Ok(Some(BackendBlock { parameters: Vec::new(), id, instructions }))
            }
            Stmt::Return { value: Some(expression @ Expr::Case { .. }), span } => {
                self.lower_case_return(id, expression, *span, instructions).map(Some)
            }
            _ => Ok(None),
        }
    }

    fn finish_block(
        &self,
        id: BackendBlockId,
        span: SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> BackendBlock {
        if !terminates(instructions.last()) {
            if let Some(target) = fallthrough {
                instructions.push(BackendInstruction::Jump { target, arguments: Vec::new(), span });
            }
        }
        BackendBlock { parameters: Vec::new(), id, instructions }
    }

    fn lower_nested_block_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        nested: &Block,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let nested_id = self.allocate_block();
        let continuation_id = self.allocate_block();
        instructions.push(BackendInstruction::Jump {
            target: nested_id,
            arguments: Vec::new(),
            span: nested.span,
        });
        let nested_block = self.lower_block(nested_id, nested, Some(continuation_id))?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let continuation_block =
            self.lower_block_in_scope(continuation_id, &continuation, fallthrough)?;
        self.blocks.push(nested_block);
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn lower_loop_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        nested: &Block,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let header_id = self.allocate_block();
        let body_id = self.allocate_block();
        let continuation_id = self.allocate_block();
        instructions.push(BackendInstruction::Jump {
            target: header_id,
            arguments: Vec::new(),
            span: nested.span,
        });
        let header = BackendBlock {
            parameters: Vec::new(),
            id: header_id,
            instructions: vec![BackendInstruction::Jump {
                target: body_id,
                arguments: Vec::new(),
                span: nested.span,
            }],
        };
        self.loop_targets.push(LoopTargets {
            continue_target: header_id,
            break_target: continuation_id,
            continue_values: Vec::new(),
            break_values: Vec::new(),
        });
        self.provider.enter_loop();
        let body_result = self.lower_block(body_id, nested, Some(header_id));
        self.provider.exit_loop();
        self.loop_targets.pop();
        let body = body_result?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let continuation_block =
            self.lower_block_in_scope(continuation_id, &continuation, fallthrough)?;
        self.blocks.push(header);
        self.blocks.push(body);
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn lower_statement(
        &mut self,
        statement: &Stmt,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        match statement {
            Stmt::OwnerDecl { name, ty, initializer, span, .. } => {
                let value = self.provider.lower_expression(initializer, instructions)?;
                self.provider.bind_owner(name, value, ty.as_deref(), *span, instructions)?;
            }
            Stmt::Assignment { target, value, span } => {
                self.lower_assignment(target, value, *span, instructions)?;
            }
            Stmt::CompoundAssignment { target, operator, value, span } => {
                self.lower_compound_assignment(target, *operator, value, *span, instructions)?;
            }
            Stmt::Return { value, span } => {
                let Some(expression) = value else {
                    instructions.push(BackendInstruction::Return { value: None, span: *span });
                    return Ok(());
                };
                if let Some(failure) = self.provider.failure_for(expression)? {
                    instructions.push(BackendInstruction::Fail { failure, span: *span });
                } else {
                    let value = self.provider.lower_expression(expression, instructions)?;
                    let value = self.provider.coerce_return_value(value, *span, instructions)?;
                    instructions
                        .push(BackendInstruction::Return { value: Some(value), span: *span });
                }
            }
            Stmt::Expression {
                expression: Expr::Call { callee, arguments, span },
                span: stmt_span,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.provider.lower_expression(&argument.expression, instructions)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let callee = self.provider.callee_for(callee, *span)?;
                instructions.push(BackendInstruction::Call {
                    callee,
                    arguments,
                    result: None,
                    span: *stmt_span,
                });
            }
            Stmt::Break { span } | Stmt::Continue { span } => {
                self.lower_loop_control(statement, *span, instructions)?;
            }
            Stmt::Drop { name, span } => {
                self.provider.drop_binding(name, *span, instructions)?;
            }
            _ => return Err(unsupported(statement_span(statement), unsupported_name(statement))),
        }
        Ok(())
    }

    fn lower_loop_control(
        &mut self,
        statement: &Stmt,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        let Some(targets) = self.loop_targets.last().cloned() else {
            return Err(unsupported(span, "loop control"));
        };
        let kind = if matches!(statement, Stmt::Continue { .. }) {
            LoopExitKind::Continue
        } else {
            LoopExitKind::Break
        };
        self.provider.emit_loop_cleanup(span, kind.clone(), instructions)?;
        let (target, names) = if kind == LoopExitKind::Continue {
            (targets.continue_target, targets.continue_values)
        } else {
            (targets.break_target, targets.break_values)
        };
        let arguments = self.loop_edge_arguments(&names, span)?;
        instructions.push(BackendInstruction::Jump { target, arguments, span });
        Ok(())
    }

    fn loop_edge_arguments(
        &mut self,
        names: &[String],
        span: SourceSpan,
    ) -> Result<Vec<BackendEdgeArgument>, BackendContractError> {
        names
            .iter()
            .map(|name| {
                let expression = Expr::Identifier { name: name.clone(), span };
                let value = self.provider.lower_expression(&expression, &mut Vec::new())?;
                let ty = self
                    .provider
                    .value_type(value)
                    .ok_or_else(|| unsupported(span, "loop binding has no value type"))?;
                Ok(BackendEdgeArgument { value, ty, span })
            })
            .collect()
    }

    fn lower_assignment(
        &mut self,
        target: &Place,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        match target {
            Place::Binding { name, .. } => {
                let value = self.provider.lower_expression(value, instructions)?;
                self.provider.assign_owner(name, value, span)
            }
            Place::Field { object, field, .. } => {
                self.provider.assign_field(&object.to_expr(), field, value, span, instructions)
            }
            Place::Index { target, index, .. } => {
                self.provider.assign_index(&target.to_expr(), index, value, span, instructions)
            }
        }
    }

    fn lower_compound_assignment(
        &mut self,
        target: &Place,
        operator: CompoundAssignmentOp,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        let Place::Binding { name, span: target_span } = target else {
            let Place::Field { object, field, .. } = target else {
                return Err(unsupported(span, "compound assignment target"));
            };
            return self.provider.compound_assign_field(
                &object.to_expr(),
                field,
                operator,
                value,
                span,
                instructions,
            );
        };
        let current = self.provider.lower_expression(
            &Place::Binding { name: name.clone(), span: *target_span }.to_expr(),
            instructions,
        )?;
        let value = self.provider.lower_expression(value, instructions)?;
        let operation = match operator {
            CompoundAssignmentOp::Add => BackendIntegerBinaryOp::Add,
            CompoundAssignmentOp::Subtract => BackendIntegerBinaryOp::Subtract,
            CompoundAssignmentOp::Multiply => BackendIntegerBinaryOp::Multiply,
            CompoundAssignmentOp::Divide => BackendIntegerBinaryOp::Divide,
            CompoundAssignmentOp::Remainder => BackendIntegerBinaryOp::Remainder,
            CompoundAssignmentOp::ShiftLeft => BackendIntegerBinaryOp::ShiftLeft,
            CompoundAssignmentOp::ShiftRight => BackendIntegerBinaryOp::ShiftRight,
            CompoundAssignmentOp::BitwiseAnd => BackendIntegerBinaryOp::BitwiseAnd,
            CompoundAssignmentOp::BitwiseOr => BackendIntegerBinaryOp::BitwiseOr,
            CompoundAssignmentOp::BitwiseXor => BackendIntegerBinaryOp::BitwiseXor,
        };
        let result =
            self.provider.lower_integer_binary(operation, current, value, span, instructions)?;
        self.provider.assign_owner(name, result, span)
    }

    fn lower_if(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&IfBranch>,
        span: SourceSpan,
        continuation_id: BackendBlockId,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<bool, BackendContractError> {
        let condition = self.provider.lower_expression(condition, instructions)?;
        let then_id = self.allocate_block();
        let else_id = self.allocate_block();
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: then_id,
            else_block: else_id,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span,
        });
        let then_block = self.lower_block(then_id, then_branch, Some(continuation_id))?;
        let then_terminates = terminates(then_block.instructions.last());
        self.blocks.push(then_block);
        let else_block = match else_branch {
            None => BackendBlock { parameters: Vec::new(), id: else_id, instructions: Vec::new() },
            Some(IfBranch::Block(block)) => {
                self.lower_block(else_id, block, Some(continuation_id))?
            }
            Some(IfBranch::ElseIf(_)) => return Err(unsupported(span, "else-if expression")),
        };
        let else_block = ensure_jump(else_block, continuation_id, span);
        let branches_terminate = then_terminates && terminates(else_block.instructions.last());
        self.blocks.push(else_block);
        Ok(branches_terminate)
    }

    pub(super) fn allocate_block(&mut self) -> BackendBlockId {
        let id = BackendBlockId(self.next_block);
        self.next_block += 1;
        id
    }
}

fn ensure_jump(mut block: BackendBlock, target: BackendBlockId, span: SourceSpan) -> BackendBlock {
    if !terminates(block.instructions.last()) {
        block.instructions.push(BackendInstruction::Jump { target, arguments: Vec::new(), span });
    }
    block
}

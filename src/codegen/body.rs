use crate::ast::{Block, CompoundAssignmentOp, Expr, IfBranch, Place, Stmt, VerbDecl};
use crate::lexer::SourceSpan;

use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendFailure, BackendInstruction,
    BackendIntegerBinaryOp, BackendTargetContext, BackendValueId, SymbolIdentity,
};

pub trait BackendBodyProvider {
    fn value_for(&self, expression: &Expr) -> Result<BackendValueId, BackendContractError>;

    fn failure_for(
        &self,
        _expression: &Expr,
    ) -> Result<Option<BackendFailure>, BackendContractError> {
        Ok(None)
    }

    fn callee_for(
        &self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError>;
}

pub trait BackendExpressionProvider {
    fn enter_scope(&mut self) {}

    fn exit_scope(&mut self) {}

    fn bind_owner(
        &mut self,
        _name: &str,
        _value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "owner declaration"))
    }

    fn assign_owner(
        &mut self,
        _name: &str,
        _value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "assignment"))
    }

    fn lower_integer_binary(
        &mut self,
        _operation: BackendIntegerBinaryOp,
        _left: BackendValueId,
        _right: BackendValueId,
        span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        Err(unsupported(span, "integer binary operation"))
    }

    fn lower_expression(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError>;

    fn failure_for(
        &mut self,
        _expression: &Expr,
    ) -> Result<Option<BackendFailure>, BackendContractError> {
        Ok(None)
    }

    fn callee_for(
        &mut self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError>;
}

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
    let mut lowering = BodyLowering { provider, next_block: 1, blocks: Vec::new() };
    let entry = BackendBlockId(0);
    let entry_block = lowering.lower_block(entry, &verb.body, None)?;
    lowering.blocks.insert(0, entry_block);
    Ok(lowering.blocks)
}

struct BodyLowering<'a, P> {
    provider: &'a mut P,
    next_block: u32,
    blocks: Vec<BackendBlock>,
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
    fn lower_block(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        fallthrough: Option<BackendBlockId>,
    ) -> Result<BackendBlock, BackendContractError> {
        self.provider.enter_scope();
        let result = self.lower_block_in_scope(id, block, fallthrough);
        self.provider.exit_scope();
        result
    }

    fn lower_block_in_scope(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        fallthrough: Option<BackendBlockId>,
    ) -> Result<BackendBlock, BackendContractError> {
        let mut instructions = Vec::new();
        for (index, statement) in block.statements.iter().enumerate() {
            if let Stmt::If { condition, then_branch, else_branch, span } = statement {
                let continuation_id = self.allocate_block();
                let branches_terminate = self.lower_if(
                    condition,
                    then_branch,
                    else_branch.as_ref(),
                    *span,
                    continuation_id,
                    &mut instructions,
                )?;
                if branches_terminate {
                    return Ok(BackendBlock { id, instructions });
                }
                let continuation =
                    Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
                let continuation_block =
                    self.lower_block(continuation_id, &continuation, fallthrough)?;
                self.blocks.push(continuation_block);
                return Ok(BackendBlock { id, instructions });
            }
            self.lower_statement(statement, &mut instructions)?;
            if terminates(instructions.last()) {
                break;
            }
        }
        if !terminates(instructions.last()) {
            if let Some(target) = fallthrough {
                instructions.push(BackendInstruction::Jump { target, span: block.span });
            }
        }
        Ok(BackendBlock { id, instructions })
    }

    fn lower_statement(
        &mut self,
        statement: &Stmt,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        match statement {
            Stmt::OwnerDecl { name, initializer, span, .. } => {
                let value = self.provider.lower_expression(initializer, instructions)?;
                self.provider.bind_owner(name, value, *span)?;
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
            _ => return Err(unsupported(statement_span(statement), unsupported_name(statement))),
        }
        Ok(())
    }

    fn lower_assignment(
        &mut self,
        target: &Place,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        let Place::Binding { name, .. } = target else {
            return Err(unsupported(span, "assignment target"));
        };
        let value = self.provider.lower_expression(value, instructions)?;
        self.provider.assign_owner(name, value, span)
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
            return Err(unsupported(span, "compound assignment target"));
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
            span,
        });
        let then_block = self.lower_block(then_id, then_branch, Some(continuation_id))?;
        let then_terminates = terminates(then_block.instructions.last());
        self.blocks.push(then_block);
        let else_block = match else_branch {
            None => BackendBlock { id: else_id, instructions: Vec::new() },
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

    fn allocate_block(&mut self) -> BackendBlockId {
        let id = BackendBlockId(self.next_block);
        self.next_block += 1;
        id
    }
}

fn ensure_jump(mut block: BackendBlock, target: BackendBlockId, span: SourceSpan) -> BackendBlock {
    if !terminates(block.instructions.last()) {
        block.instructions.push(BackendInstruction::Jump { target, span });
    }
    block
}

fn terminates(instruction: Option<&BackendInstruction>) -> bool {
    matches!(
        instruction,
        Some(BackendInstruction::Branch { .. })
            | Some(BackendInstruction::Jump { .. })
            | Some(BackendInstruction::Fail { .. })
            | Some(BackendInstruction::Return { .. })
            | Some(BackendInstruction::Trap { .. })
    )
}

fn statement_span(statement: &Stmt) -> SourceSpan {
    match statement {
        Stmt::OwnerDecl { span, .. }
        | Stmt::Assignment { span, .. }
        | Stmt::CompoundAssignment { span, .. }
        | Stmt::Expression { span, .. }
        | Stmt::If { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::ForRange { span, .. }
        | Stmt::ForArray { span, .. }
        | Stmt::Break { span }
        | Stmt::Continue { span }
        | Stmt::Drop { span, .. } => *span,
        Stmt::Loop(block) | Stmt::Block(block) => block.span,
    }
}

fn unsupported_name(statement: &Stmt) -> &'static str {
    match statement {
        Stmt::OwnerDecl { .. } => "owner declaration",
        Stmt::Assignment { .. } => "assignment",
        Stmt::CompoundAssignment { .. } => "compound assignment",
        Stmt::Expression { .. } => "expression",
        Stmt::If { .. } => "if statement",
        Stmt::Return { .. } => "return statement",
        Stmt::Loop(_) => "loop",
        Stmt::ForRange { .. } => "range loop",
        Stmt::ForArray { .. } => "array loop",
        Stmt::Break { .. } => "break",
        Stmt::Continue { .. } => "continue",
        Stmt::Drop { .. } => "drop statement",
        Stmt::Block(_) => "nested block",
    }
}

fn unsupported(span: SourceSpan, name: &str) -> BackendContractError {
    BackendContractError::Unsupported(super::BackendUnsupportedOperation::new(
        "body-lowering",
        span,
        BackendTargetContext {
            backend: "neutral-contract".to_owned(),
            target: "semantic-model".to_owned(),
        },
        format!("{name} has no neutral lowering contract yet"),
    ))
}

//! A `?` among the statements of a value arm (bug #130): the arm of an `if` or
//! a `case` whose value an owner receives is lowered statement by statement
//! into the block that ends with its value, but a `?` leaves that block. The
//! semantic normalization makes every such `?` a statement of the arm
//! (`erg v = load()?;`), and this lowering splits the arm there, as a `?`
//! statement of any other block does: the `Err` path returns from the verb with
//! the exit list the analyzer planned for the `?` (ADR-0125), and the `Ok`
//! path continues the arm in a new block that receives the payload, so the
//! arm's value reaches the merge from the last block of the arm.

use super::try_value::{append_try_condition_branch, exit_return};
use super::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendContractError, BackendEdgeArgument,
    BackendExpressionProvider, BackendInstruction, BodyLowering,
};
use crate::ast::{Expr, Stmt};
use crate::lexer::SourceSpan;

/// The block in which the lowering of a value arm continues: the arm's first
/// block, or the block that the `Ok` path of its last `?` entered, with the
/// payload parameter of that block. `diverged` holds when the arm returned
/// from the verb (`return load()?;`), so it produces no value.
pub(super) struct ArmTail {
    pub(super) id: BackendBlockId,
    pub(super) parameters: Vec<BackendBlockParameter>,
    pub(super) instructions: Vec<BackendInstruction>,
    pub(super) diverged: bool,
}

impl ArmTail {
    /// The arm's current block with its instructions, as it stands.
    pub(super) fn into_block(self) -> BackendBlock {
        BackendBlock { parameters: self.parameters, id: self.id, instructions: self.instructions }
    }
}

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    /// Lowers `statements` of a value arm, starting in block `id` with
    /// `instructions`, and returns the block in which the arm continues. Each
    /// `?` statement (`erg v = e?;`, `x = e?;`, `e?;`, `return e?;`) ends the
    /// current block with the test of its `Result`; every other statement is
    /// lowered in place.
    pub(super) fn lower_arm_statements(
        &mut self,
        id: BackendBlockId,
        statements: &[Stmt],
        instructions: Vec<BackendInstruction>,
    ) -> Result<ArmTail, BackendContractError> {
        let mut tail = ArmTail { id, parameters: Vec::new(), instructions, diverged: false };
        for statement in statements {
            match statement {
                Stmt::OwnerDecl {
                    name, ty, initializer: Expr::Try { expression, span }, ..
                } => {
                    self.split_arm_try(&mut tail, expression, *span, Some((name, ty.as_deref())))?;
                }
                Stmt::Expression { expression: Expr::Try { expression, span }, .. } => {
                    self.split_arm_try(&mut tail, expression, *span, None)?;
                }
                Stmt::Assignment { target, value: Expr::Try { expression, span }, span: at } => {
                    // The payload is bound to a synthetic owner first, as for
                    // a `?` assignment of any other block.
                    let name = format!("__try_assignment_{}_{}", span.start, span.end);
                    self.split_arm_try(&mut tail, expression, *span, Some((&name, None)))?;
                    let read = Expr::Identifier { name, span: *at };
                    let assignment =
                        Stmt::Assignment { target: target.clone(), value: read, span: *at };
                    self.lower_statement(&assignment, &mut tail.instructions)?;
                }
                Stmt::Return { value: Some(Expr::Try { expression, span }), span: exit } => {
                    let instructions = std::mem::take(&mut tail.instructions);
                    let block = self.lower_try_return_statement(
                        tail.id,
                        *exit,
                        expression,
                        *span,
                        instructions,
                    )?;
                    tail.instructions = block.instructions;
                    tail.diverged = true;
                    return Ok(tail);
                }
                _ => self.lower_statement(statement, &mut tail.instructions)?,
            }
        }
        Ok(tail)
    }

    /// Ends the current block of `tail` with the test of the `Result` that
    /// `expression` produces: the `Err` path returns it from the verb after the
    /// cleanup planned for the `?` at `span`, and the `Ok` path enters a new
    /// block that receives the payload and binds it to `owner` when there is
    /// one; `tail` then continues in that block.
    fn split_arm_try(
        &mut self,
        tail: &mut ArmTail,
        expression: &Expr,
        span: SourceSpan,
        owner: Option<(&str, Option<&str>)>,
    ) -> Result<(), BackendContractError> {
        let source = self.provider.lower_expression(expression, &mut tail.instructions)?;
        let condition = self.provider.lower_enum_variant_condition_value(
            source,
            "Ok",
            span,
            &mut tail.instructions,
        )?;
        let ok_block = self.allocate_block();
        let error_block = self.allocate_block();
        let next = self.allocate_block();
        let mut ok_instructions = Vec::new();
        let mut parameters = Vec::new();
        let mut arguments = Vec::new();
        let mut instructions = Vec::new();
        if let Some((name, declared_type)) = owner {
            let (payload_instructions, payload, payload_type) =
                self.lower_try_payload(source, span)?;
            ok_instructions = payload_instructions;
            let received = self.provider.allocate_value(payload_type);
            parameters.push(BackendBlockParameter { value: received, ty: payload_type, span });
            arguments.push(BackendEdgeArgument { value: payload, ty: payload_type, span });
            self.provider.bind_owner(name, received, declared_type, span, &mut instructions)?;
        }
        ok_instructions.push(BackendInstruction::Jump { target: next, arguments, span });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: ok_block,
            instructions: ok_instructions,
        });
        let error_instructions = exit_return(self.provider, source, span, span)?;
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: error_block,
            instructions: error_instructions,
        });
        append_try_condition_branch(&mut tail.instructions, condition, ok_block, error_block, span);
        let finished = std::mem::replace(
            tail,
            ArmTail { id: next, parameters, instructions, diverged: false },
        );
        self.blocks.push(finished.into_block());
        Ok(())
    }
}

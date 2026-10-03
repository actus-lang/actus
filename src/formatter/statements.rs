use crate::ast::{Block, CompoundAssignmentOp, Place, Stmt};

use super::{Formatter, role_name};

impl Formatter<'_> {
    pub(super) fn block(&mut self, block: &Block) {
        self.output.push('{');
        if block.statements.is_empty() {
            self.output.push('}');
            return;
        }

        self.output.push('\n');
        self.indent += 1;
        for statement in &block.statements {
            self.emit_comments_before(statement_span(statement).start);
            self.line_indent();
            self.statement(statement);
            self.output.push('\n');
        }
        self.emit_comments_before(block.span.end);
        self.indent -= 1;
        self.line_indent();
        self.output.push('}');
    }

    fn statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::OwnerDecl { role, name, ty, initializer, .. } => {
                self.owner_declaration(role, name, ty.as_deref(), initializer);
            }
            Stmt::Assignment { target, value, .. } => {
                self.place(target);
                self.output.push_str(" = ");
                self.expression(value);
                self.output.push(';');
            }
            Stmt::CompoundAssignment { target, operator, value, .. } => {
                self.compound_assignment(target, *operator, value);
            }
            Stmt::Expression { expression, .. } => {
                self.expression(expression);
                self.output.push(';');
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                self.if_expression(condition, then_branch, else_branch.as_ref());
            }
            Stmt::Return { value, .. } => {
                self.return_statement(value.as_ref());
            }
            Stmt::Loop(block) => {
                self.output.push_str("loop ");
                self.block(block);
            }
            Stmt::Break { .. } => self.output.push_str("break;"),
            Stmt::Continue { .. } => self.output.push_str("continue;"),
            Stmt::Drop { name, .. } => {
                self.output.push_str("drop(");
                self.output.push_str(name);
                self.output.push_str(");");
            }
            Stmt::Block(block) => self.block(block),
        }
    }

    fn compound_assignment(
        &mut self,
        target: &Place,
        operator: CompoundAssignmentOp,
        value: &crate::ast::Expr,
    ) {
        match target {
            Place::Binding { name, .. } => self.output.push_str(name),
            Place::Field { object, field, .. } => {
                self.place(object);
                self.output.push('.');
                self.output.push_str(field);
            }
            Place::Index { target, index, .. } => {
                self.place(target);
                self.output.push('[');
                self.expression(index);
                self.output.push(']');
            }
        }
        self.output.push_str(compound_operator_text(operator));
        self.expression(value);
        self.output.push(';');
    }

    fn owner_declaration(
        &mut self,
        role: &crate::ast::Role,
        name: &str,
        ty: Option<&str>,
        initializer: &crate::ast::Expr,
    ) {
        self.output.push_str(role_name(role));
        self.output.push(' ');
        self.output.push_str(name);
        if let Some(ty) = ty {
            self.output.push_str(": ");
            self.output.push_str(ty);
        }
        self.output.push_str(" = ");
        self.expression(initializer);
        self.output.push(';');
    }

    fn place(&mut self, place: &Place) {
        match place {
            Place::Binding { name, .. } => self.output.push_str(name),
            Place::Field { object, field, .. } => {
                self.place(object);
                self.output.push('.');
                self.output.push_str(field);
            }
            Place::Index { target, index, .. } => {
                self.place(target);
                self.output.push('[');
                self.expression(index);
                self.output.push(']');
            }
        }
    }

    fn return_statement(&mut self, value: Option<&crate::ast::Expr>) {
        self.output.push_str("return");
        if let Some(value) = value {
            self.output.push(' ');
            self.expression(value);
        }
        self.output.push(';');
    }
}

fn statement_span(statement: &Stmt) -> crate::lexer::SourceSpan {
    match statement {
        Stmt::OwnerDecl { span, .. }
        | Stmt::Assignment { span, .. }
        | Stmt::CompoundAssignment { span, .. }
        | Stmt::Expression { span, .. }
        | Stmt::If { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Break { span }
        | Stmt::Continue { span }
        | Stmt::Drop { span, .. } => *span,
        Stmt::Loop(block) | Stmt::Block(block) => block.span,
    }
}

fn compound_operator_text(operator: CompoundAssignmentOp) -> &'static str {
    match operator {
        CompoundAssignmentOp::Add => " += ",
        CompoundAssignmentOp::Subtract => " -= ",
        CompoundAssignmentOp::Multiply => " *= ",
        CompoundAssignmentOp::Divide => " /= ",
        CompoundAssignmentOp::Remainder => " %= ",
        CompoundAssignmentOp::BitwiseAnd => " &= ",
        CompoundAssignmentOp::BitwiseOr => " |= ",
        CompoundAssignmentOp::BitwiseXor => " ^= ",
        CompoundAssignmentOp::ShiftLeft => " <<= ",
        CompoundAssignmentOp::ShiftRight => " >>= ",
    }
}

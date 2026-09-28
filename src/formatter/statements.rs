use crate::ast::{Block, Stmt};

use super::{Formatter, role_name};

impl Formatter {
    pub(super) fn block(&mut self, block: &Block) {
        self.output.push('{');
        if block.statements.is_empty() {
            self.output.push('}');
            return;
        }

        self.output.push('\n');
        self.indent += 1;
        for statement in &block.statements {
            self.line_indent();
            self.statement(statement);
            self.output.push('\n');
        }
        self.indent -= 1;
        self.line_indent();
        self.output.push('}');
    }

    fn statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::OwnerDecl { role, name, ty, initializer, .. } => {
                self.owner_declaration(role, name, ty.as_deref(), initializer);
            }
            Stmt::Assignment { name, value, .. } => {
                self.output.push_str(name);
                self.output.push_str(" = ");
                self.expression(value);
                self.output.push(';');
            }
            Stmt::FieldAssignment { object, field, value, .. } => {
                self.field_assignment(object, field, value);
            }
            Stmt::Expression { expression, .. } => {
                self.expression(expression);
                self.output.push(';');
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

    fn field_assignment(
        &mut self,
        object: &crate::ast::Expr,
        field: &str,
        value: &crate::ast::Expr,
    ) {
        self.expression(object);
        self.output.push('.');
        self.output.push_str(field);
        self.output.push_str(" = ");
        self.expression(value);
        self.output.push(';');
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

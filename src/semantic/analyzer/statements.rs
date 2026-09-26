use crate::ast::{Block, Expr, Role, Stmt};
use crate::lexer::SourceSpan;

use super::super::errors::SemanticError;
use super::Analyzer;
use super::is_origin_return_expression;

impl Analyzer {
    pub(crate) fn visit_block(&mut self, block: &Block) -> Result<(), SemanticError> {
        for statement in &block.statements {
            self.visit_statement(statement)?;
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Stmt) -> Result<(), SemanticError> {
        match statement {
            Stmt::OwnerDecl { role, name, ty, initializer, span } => {
                self.visit_owner_decl(role, name, ty.as_deref(), initializer, *span)
            }
            Stmt::Assignment { name, value, span } => {
                let index = self.binding(name, *span)?;
                self.ensure_mutable(index, name, *span)?;
                self.validate_binding_assignment(index, name, value, *span)?;
                self.visit_expression(value)
            }
            Stmt::FieldAssignment { object, field, value, span } => {
                self.validate_field_assignment(object, field, value, *span)
            }
            Stmt::Expression { expression, span } => self.visit_expr_statement(expression, *span),
            Stmt::Return { value, span } => self.visit_return(value.as_ref(), *span),
            Stmt::Loop(block) => {
                self.enter_scope(block.span);
                self.loop_boundaries.push(self.scopes.len() - 1);
                self.visit_block(block)?;
                self.loop_boundaries.pop();
                self.leave_scope();
                Ok(())
            }
            Stmt::Break { span } => {
                self.plan_loop_unwind(super::super::cleanup::LoopExitKind::Break, "break", *span)
            }
            Stmt::Continue { span } => self.plan_loop_unwind(
                super::super::cleanup::LoopExitKind::Continue,
                "continue",
                *span,
            ),
            Stmt::Drop { name, span } => self.drop_binding(name, *span),
            Stmt::Block(block) => {
                self.enter_scope(block.span);
                self.visit_block(block)?;
                self.leave_scope();
                Ok(())
            }
        }
    }

    fn visit_owner_decl(
        &mut self,
        role: &Role,
        name: &str,
        declared_type: Option<&str>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let declared_type_name =
            declared_type.and_then(|name| super::super::calls::parse_type_name_key(name, span));
        let binding_type =
            self.resolve_binding_type(declared_type_name.as_ref(), initializer, span)?;
        self.visit_expression_with_expected(initializer, declared_type_name.as_ref())?;
        self.validate_declared_initializer(name, declared_type_name.as_ref(), initializer, span)?;
        if *role == Role::Erg {
            self.initialize_owner(initializer, span)?;
        }
        if *role == Role::Abs {
            self.register_borrow(initializer, span)?;
        }
        self.bind(role.clone(), name.to_owned(), binding_type, span)?;
        if *role == Role::Abs
            && let Ok(index) = self.binding(name, span)
        {
            let origin = self.origin_of(initializer);
            self.binding_origins.insert(index, origin.clone());
            if is_origin_return_expression(initializer) {
                self.register_origin_borrow(&origin, span)?;
            }
        }
        self.record_initializer_struct_type(name, initializer, span)?;
        if let Some(type_name) = declared_type_name {
            self.record_struct_binding(name, &type_name, span)?;
            self.record_enum_binding(name, &type_name, span)?;
        } else {
            self.record_initializer_enum_type(name, initializer, span)?;
        }
        Ok(())
    }

    fn visit_expr_statement(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(expression)?;
        if matches!(expression, Expr::Try { .. }) {
            self.plan_return_unwind(span);
        }
        Ok(())
    }
}

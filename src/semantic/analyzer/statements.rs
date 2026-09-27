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
                if let Some(expected) = self.binding_type_names.get(&index) {
                    self.validate_expected_literal(value, expected)?;
                }
                self.visit_expression(value)?;
                self.validate_arena_provenance_target(index, name, value, *span)?;
                self.record_binding_arena_provenance(index, value);
                self.plan_try_unwind(value);
                Ok(())
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
        self.visit_expression_with_expected(initializer, declared_type_name.as_ref())?;
        self.plan_try_unwind(initializer);
        let binding_type =
            self.resolve_binding_type(declared_type_name.as_ref(), initializer, span)?;
        self.validate_declared_initializer(name, declared_type_name.as_ref(), initializer, span)?;
        self.validate_arena_provenance_for_binding(name, initializer, span)?;
        if *role == Role::Erg {
            self.initialize_owner(initializer, span)?;
        }
        if *role == Role::Abs {
            self.register_borrow(initializer, span)?;
        }
        self.bind(role.clone(), name.to_owned(), binding_type, span)?;
        let binding_index = self.binding(name, span)?;
        if let Some(type_name) = declared_type_name.as_ref() {
            self.binding_type_names.insert(binding_index, type_name.clone());
        }
        self.record_binding_arena_provenance(binding_index, initializer);
        if declared_type_name.as_ref().is_some_and(|type_name| type_name.name == "Arena") {
            let provenance = self.next_arena_id;
            self.next_arena_id += 1;
            self.binding_arena_provenance.insert(binding_index, provenance);
            self.arena_scope_depth.insert(provenance, self.scopes.len());
        }
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
        self.record_initializer_pack_type(name, initializer, span)?;
        if let Some(type_name) = declared_type_name {
            self.record_struct_binding(name, &type_name, span)?;
            self.record_enum_binding(name, &type_name, span)?;
        } else {
            self.record_initializer_enum_type(name, initializer, span)?;
        }
        Ok(())
    }

    fn plan_try_unwind(&mut self, expression: &Expr) {
        if let Expr::Try { span, .. } = expression {
            self.plan_return_unwind(*span);
        }
    }

    fn record_binding_arena_provenance(&mut self, binding_index: usize, expression: &Expr) {
        self.binding_arena_provenance.remove(&binding_index);
        let arenas = self.arena_provenances(expression);
        if arenas.len() == 1 {
            self.binding_arena_provenance.insert(binding_index, *arenas.iter().next().unwrap());
        }
    }

    fn validate_arena_provenance_for_binding(
        &self,
        name: &str,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let arenas = self.arena_provenances(expression);
        if arenas.len() > 1 {
            return Err(SemanticError {
                kind: super::super::errors::SemanticErrorKind::CrossArenaReference {
                    name: name.to_owned(),
                },
                span,
            });
        }
        if let Some(arena_id) = arenas.iter().next()
            && self.scopes.len() < *self.arena_scope_depth.get(arena_id).unwrap_or(&0)
        {
            return Err(SemanticError {
                kind: super::super::errors::SemanticErrorKind::ArenaReferenceEscape {
                    name: name.to_owned(),
                },
                span,
            });
        }
        Ok(())
    }

    pub(crate) fn validate_arena_provenance_target(
        &self,
        binding_index: usize,
        name: &str,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let arenas = self.arena_provenances(expression);
        if arenas.len() > 1 {
            return Err(SemanticError {
                kind: super::super::errors::SemanticErrorKind::CrossArenaReference {
                    name: name.to_owned(),
                },
                span,
            });
        }
        if let Some(arena_id) = arenas.iter().next()
            && self.binding_scope_depth.get(&binding_index).copied().unwrap_or(0)
                < *self.arena_scope_depth.get(arena_id).unwrap_or(&0)
        {
            return Err(SemanticError {
                kind: super::super::errors::SemanticErrorKind::ArenaReferenceEscape {
                    name: name.to_owned(),
                },
                span,
            });
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

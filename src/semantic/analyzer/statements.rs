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
            Stmt::Assignment { name, value, span } => self.visit_assignment(name, value, *span),
            Stmt::FieldAssignment { object, field, value, span } => {
                self.validate_field_assignment(object, field, value, *span)
            }
            Stmt::IndexAssignment { target, index, value, .. } => {
                self.visit_index_assignment(target, index, value)
            }
            Stmt::Expression { expression, span } => self.visit_expr_statement(expression, *span),
            Stmt::Return { value, span } => self.visit_return(value.as_ref(), *span),
            Stmt::Loop(block) => self.visit_loop(block),
            Stmt::Break { span } => self.visit_loop_control(true, *span),
            Stmt::Continue { span } => self.visit_loop_control(false, *span),
            Stmt::Drop { name, span } => self.drop_binding(name, *span),
            Stmt::Block(block) => self.visit_nested_block(block),
        }
    }

    fn visit_assignment(
        &mut self,
        name: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let index = self.binding(name, span)?;
        self.ensure_mutable(index, name, span)?;
        self.validate_binding_assignment(index, name, value, span)?;
        if let Some(expected) = self.binding_type_names.get(&index) {
            self.validate_expected_literal(value, expected)?;
        }
        self.visit_expression(value)?;
        self.move_aggregate_assignment_source(name, value, span)?;
        self.validate_arena_provenance_target(index, name, value, span)?;
        self.record_binding_arena_provenance(index, value);
        self.plan_try_unwind(value);
        Ok(())
    }

    fn move_aggregate_assignment_source(
        &mut self,
        destination: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), super::super::errors::SemanticError> {
        if !self.is_move_only_aggregate(value) {
            return Ok(());
        }
        if let Expr::Identifier { name, .. } = value
            && name == destination
        {
            return Err(super::super::errors::SemanticError {
                kind: super::super::errors::SemanticErrorKind::SelfAssignment {
                    name: name.clone(),
                },
                span,
            });
        }
        if matches!(value, Expr::Identifier { .. } | Expr::FieldAccess { .. }) {
            self.move_dat_argument(value, span)?;
        }
        Ok(())
    }

    fn is_move_only_aggregate(&self, expression: &Expr) -> bool {
        if let Expr::FieldAccess { object, .. } = expression
            && self.enum_receiver_name(object).is_some()
        {
            return false;
        }
        let Some(type_name) = self.expression_type_name(expression) else { return false };
        type_name == "Buffer"
            || type_name == "Array"
            || self.struct_types.contains_key(&type_name)
            || self.enum_types.contains_key(&type_name)
    }

    fn visit_index_assignment(
        &mut self,
        target: &Expr,
        index: &Expr,
        value: &Expr,
    ) -> Result<(), SemanticError> {
        if let Expr::Identifier { name, span: target_span } = target {
            let binding = self.binding(name, *target_span)?;
            self.ensure_mutable(binding, name, *target_span)?;
        }
        self.visit_expression(target)?;
        self.visit_expression(index)?;
        self.validate_index_value(target, index, value)?;
        self.visit_expression(value)?;
        self.plan_try_unwind(value);
        Ok(())
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
        self.validate_owner_initializer(initializer, declared_type_name.as_ref())?;
        let binding_type =
            self.resolve_binding_type(declared_type_name.as_ref(), initializer, span)?;
        self.validate_owner_binding(name, declared_type_name.as_ref(), initializer, span)?;
        self.initialize_owner_binding(role, initializer, span)?;
        let binding_index = self.bind_owner_binding(role, name, binding_type, span)?;
        self.record_owner_metadata(
            role,
            name,
            declared_type_name.as_ref(),
            initializer,
            binding_index,
            span,
        )?;
        self.record_owner_type_links(name, declared_type_name, initializer, span)?;
        Ok(())
    }

    fn validate_owner_initializer(
        &mut self,
        initializer: &Expr,
        declared_type: Option<&crate::ast::TypeName>,
    ) -> Result<(), SemanticError> {
        self.visit_expression_with_expected(initializer, declared_type)?;
        self.plan_try_unwind(initializer);
        Ok(())
    }

    fn validate_owner_binding(
        &mut self,
        name: &str,
        declared_type: Option<&crate::ast::TypeName>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_declared_initializer(name, declared_type, initializer, span)?;
        self.validate_arena_provenance_for_binding(name, initializer, span)
    }

    fn initialize_owner_binding(
        &mut self,
        role: &Role,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if *role == Role::Erg {
            self.initialize_owner(initializer, span)?;
        }
        if *role == Role::Abs {
            self.register_borrow(initializer, span)?;
        }
        Ok(())
    }

    fn bind_owner_binding(
        &mut self,
        role: &Role,
        name: &str,
        binding_type: Option<crate::ast::BuiltinType>,
        span: SourceSpan,
    ) -> Result<usize, SemanticError> {
        self.bind(role.clone(), name.to_owned(), binding_type, span)?;
        self.binding(name, span)
    }

    fn record_owner_metadata(
        &mut self,
        role: &Role,
        name: &str,
        declared_type: Option<&crate::ast::TypeName>,
        initializer: &Expr,
        binding_index: usize,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Some(type_name) = declared_type {
            self.binding_type_names.insert(binding_index, type_name.clone());
        } else {
            self.record_inferred_binding_type(binding_index, initializer, span);
        }
        self.record_binding_arena_provenance(binding_index, initializer);
        self.record_arena_binding(declared_type, binding_index);
        self.record_origin_binding(role, name, initializer, span)
    }

    fn record_arena_binding(&mut self, declared_type: Option<&crate::ast::TypeName>, index: usize) {
        if declared_type.is_some_and(|type_name| type_name.name == "Arena") {
            let provenance = self.next_arena_id;
            self.next_arena_id += 1;
            self.binding_arena_provenance.insert(index, provenance);
            self.arena_scope_depth.insert(provenance, self.scopes.len());
        }
    }

    fn record_origin_binding(
        &mut self,
        role: &Role,
        name: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if *role != Role::Abs {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        let origin = self.origin_of(initializer);
        self.binding_origins.insert(index, origin.clone());
        if is_origin_return_expression(initializer) {
            self.register_origin_borrow(&origin, span)?;
        }
        Ok(())
    }

    fn record_owner_type_links(
        &mut self,
        name: &str,
        declared_type: Option<crate::ast::TypeName>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.record_initializer_struct_type(name, initializer, span)?;
        self.record_initializer_pack_type(name, initializer, span)?;
        if let Some(type_name) = declared_type {
            self.record_struct_binding(name, &type_name, span)?;
            self.record_enum_binding(name, &type_name, span)
        } else {
            self.record_initializer_enum_type(name, initializer, span)
        }
    }

    fn record_inferred_binding_type(
        &mut self,
        binding_index: usize,
        initializer: &Expr,
        span: SourceSpan,
    ) {
        let Some(type_name) = self.expression_type_name(initializer) else { return };
        let Some(type_name) = super::super::calls::parse_type_name_key(&type_name, span) else {
            return;
        };
        self.binding_type_names.insert(binding_index, type_name);
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

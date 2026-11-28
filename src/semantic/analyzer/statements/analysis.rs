use crate::ast::{Block, Expr, Place, Role, Stmt, TypeName};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::is_origin_return_expression;
use super::{Analyzer, type_name_text_contains_float};

impl Analyzer {
    pub(crate) fn visit_block(&mut self, block: &Block) -> Result<(), SemanticError> {
        for statement in &block.statements {
            self.visit_statement(statement)?;
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Stmt) -> Result<(), SemanticError> {
        match statement {
            Stmt::OwnerDecl { role, name, ty, resolved_type, initializer, span } => {
                if (self.current_realtime || self.current_deterministic)
                    && ty.as_ref().is_some_and(|name| type_name_text_contains_float(name))
                {
                    return Err(SemanticError {
                        kind: if self.current_deterministic {
                            SemanticErrorKind::DeterministicFloatingPoint
                        } else {
                            SemanticErrorKind::RealtimeFloatingPoint
                        },
                        span: *span,
                    });
                }
                self.visit_owner_decl(role, name, resolved_type.as_ref(), initializer, *span)
            }
            Stmt::Assignment { target, value, span } => {
                self.visit_place_assignment(target, value, *span)
            }
            Stmt::CompoundAssignment { target, operator, value, span } => {
                self.visit_compound_assignment(target, *operator, value, *span)
            }
            Stmt::Expression { expression, span } => {
                self.visit_expr_statement(expression)?;
                self.release_discarded_value(expression, *span);
                Ok(())
            }
            Stmt::If { condition, then_branch, else_branch, span } => {
                let expression = Expr::If {
                    condition: Box::new(condition.clone()),
                    then_branch: then_branch.clone(),
                    else_branch: else_branch.clone(),
                    span: *span,
                };
                let expected = self.expected_expression_type.take();
                let result = self.visit_if_expression(&expression);
                self.expected_expression_type = expected;
                result
            }
            Stmt::Return { value, span } => self.visit_return(value.as_ref(), *span),
            Stmt::Loop(block) => self.visit_loop(block),
            Stmt::ForRange { binding, start, end, body, span } => {
                self.visit_for_range(binding, start, end, body, *span)
            }
            Stmt::ForArray { binding, collection, body, span } => {
                self.visit_for_array(binding, collection, body, *span)
            }
            Stmt::Break { span } => self.visit_loop_control(true, *span),
            Stmt::Continue { span } => self.visit_loop_control(false, *span),
            Stmt::Drop { name, span } => self.drop_binding(name, *span),
            Stmt::Block(block) => self.visit_nested_block(block),
        }
    }

    fn visit_compound_assignment(
        &mut self,
        target: &Place,
        operator: crate::ast::CompoundAssignmentOp,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let binary_operator = operator.binary_operator();
        match target {
            Place::Binding { name, span: target_span } => {
                self.reject_constant_assignment(name, *target_span)?;
                let binding = self.binding(name, span)?;
                self.ensure_mutable(binding, name, span)?;
                let left = Expr::Identifier { name: name.clone(), span };
                self.visit_expression(value)?;
                self.validate_binary_operator(binary_operator, &left, value)
            }
            Place::Field { object, field, .. } => {
                let object_expression = object.to_expr();
                let left = Expr::FieldAccess {
                    object: Box::new(object_expression.clone()),
                    field: field.clone(),
                    resolved_type: None,
                    span,
                };
                self.precheck_compound_operands(binary_operator, &left, value)?;
                self.validate_field_assignment(&object_expression, field, value, span)?;
                self.validate_binary_operator(binary_operator, &left, value)
            }
            Place::Index { target, index, .. } => {
                let target_expression = target.to_expr();
                let left = Expr::Index {
                    target: Box::new(target_expression.clone()),
                    index: index.clone(),
                    span,
                };
                self.precheck_compound_operands(binary_operator, &left, value)?;
                self.visit_index_assignment(&target_expression, index, value, span)?;
                self.validate_binary_operator(binary_operator, &left, value)
            }
        }
    }

    /// Reports a mixed-type compound assignment to a field or element as the
    /// operator error (`E1140`) before the place's own type check reports the
    /// same difference as a plain assignment mismatch. Silent when either
    /// type is unknown, so the place's own diagnostics stay in charge.
    fn precheck_compound_operands(
        &self,
        operator: crate::ast::BinaryOp,
        place: &Expr,
        value: &Expr,
    ) -> Result<(), SemanticError> {
        if self.expression_type_name(place).is_some() && self.expression_type_name(value).is_some()
        {
            return self.validate_binary_operator(operator, place, value);
        }
        Ok(())
    }

    fn visit_place_assignment(
        &mut self,
        target: &Place,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        match target {
            Place::Binding { name, span: target_span } => {
                self.reject_constant_assignment(name, *target_span)?;
                self.visit_assignment(name, value, span)
            }
            Place::Field { object, field, .. } => {
                let object = object.to_expr();
                self.validate_field_assignment(&object, field, value, span)
            }
            Place::Index { target, index, .. } => {
                let target = target.to_expr();
                self.visit_index_assignment(&target, index, value, span)
            }
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
        self.ensure_arena_references_inactive(index, name, span)?;
        self.reject_void_value(value)?;
        self.validate_binding_assignment(index, name, value, span)?;
        if let Some(expected) = self.binding_type_names.get(&index) {
            self.validate_expected_literal(value, expected)?;
        }
        self.visit_expression(value)?;
        self.move_aggregate_assignment_source(name, value, span)?;
        self.validate_arena_provenance_target(index, name, value, span)?;
        self.record_binding_provenance(index, value)?;
        self.plan_try_unwind(value);
        self.replace_owned_value(index, span);
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
        if matches!(value, Expr::Index { .. }) {
            self.reject_move_out_of_view(value, span)?;
        }
        if matches!(value, Expr::Identifier { .. } | Expr::FieldAccess { .. }) {
            self.move_dat_argument(value, span)?;
        }
        Ok(())
    }

    pub(in crate::semantic) fn is_move_only_aggregate(&self, expression: &Expr) -> bool {
        if let Expr::FieldAccess { object, .. } = expression
            && self.enum_receiver_name(object).is_some()
        {
            return false;
        }
        let Some(type_name) = self.expression_type_name(expression) else { return false };
        // A generic instance such as `Array[Int, 4]` or `Option[Buffer]` is
        // move-only like its declaration, so only the head of the name counts.
        let head = type_name.split('[').next().unwrap_or(&type_name);
        matches!(head, "Buffer" | "Arena" | "Array")
            || self.struct_types.contains_key(head)
            || self.enum_types.contains_key(head)
            || self.pack_types.contains_key(head)
    }

    fn visit_owner_decl(
        &mut self,
        role: &Role,
        name: &str,
        declared_type: Option<&TypeName>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let mut declared_type_name = declared_type.cloned();
        if let Some(type_name) = &mut declared_type_name {
            self.normalize_array_capacities(type_name)?;
        }
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
            // A struct or enum read from a field moves by its field path.
            self.consume_owned_source(initializer, span)?;
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
            self.binding_type_names
                .insert(binding_index, self.normalize_imported_type_name(type_name));
        } else {
            self.record_inferred_binding_type(binding_index, initializer, span);
        }
        self.record_binding_provenance(binding_index, initializer)?;
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

    pub(super) fn plan_try_unwind(&mut self, expression: &Expr) {
        if let Expr::Try { span, .. } = expression {
            self.plan_return_unwind(*span);
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

    fn visit_expr_statement(&mut self, expression: &Expr) -> Result<(), SemanticError> {
        let previous = self.discarded_expression;
        let previous_consumed = self.expression_value_consumed;
        self.discarded_expression = true;
        self.expression_value_consumed = false;
        self.drops_next_value = true;
        let result = self.visit_expression(expression);
        self.discarded_expression = previous;
        self.expression_value_consumed = previous_consumed;
        result?;
        // A bare literal statement, such as the tail of an `if` branch, is an `Int`.
        self.validate_default_integer_literal(expression)?;
        // The failure exit of a `?` statement is planned under the span of
        // the `?` expression, as every other `?` exit is (ADR-0125).
        self.plan_try_unwind(expression);
        Ok(())
    }
}

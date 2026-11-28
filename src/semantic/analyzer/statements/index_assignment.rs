use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::errors::SemanticError;
use super::Analyzer;

impl Analyzer {
    /// Analyses the assignment `target[index] = value`: validates the element
    /// and the value, records the views the value adds to the array's owner
    /// (ADR-0046 2.3), and moves a struct, enum or resource into the element
    /// (ADR-0124).
    pub(super) fn visit_index_assignment(
        &mut self,
        target: &Expr,
        index: &Expr,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_index_assignment_owner(target)?;
        self.visit_expression(target)?;
        self.visit_expression(index)?;
        self.reject_void_value(value)?;
        let element = self.validate_index_access(target, index)?;
        // Visit the value first: an explicit `copy` is typed by its operand.
        self.visit_expression(value)?;
        self.validate_index_value(target, index, value)?;
        self.reject_move_out_of_view(value, super::super::validation::expression_span(value))?;
        self.record_element_view_provenance(target, value)?;
        // A struct, enum or resource stored in an element moves (ADR-0124).
        if self.argument_moves_source(value, &element) {
            self.consume_owned_source(value, super::super::validation::expression_span(value))?;
        }
        self.plan_try_unwind(value);
        self.replace_owned_element(target, &element, span);
        Ok(())
    }

    fn validate_index_assignment_owner(&self, target: &Expr) -> Result<(), SemanticError> {
        match target {
            Expr::Identifier { name, span } => {
                self.reject_constant_assignment(name, *span)?;
                let binding = self.binding(name, *span)?;
                self.ensure_mutable(binding, name, *span)
            }
            Expr::FieldAccess { object, field, span, .. } => {
                self.validate_index_assignment_owner(object)?;
                if let Some(pack_name) = self.expression_pack_type(object) {
                    let Some(pack_field) = self.pack_field(&pack_name, field) else {
                        return Ok(());
                    };
                    if matches!(pack_field.role, crate::ast::Role::Abs) {
                        return Err(SemanticError {
                            kind: super::super::errors::SemanticErrorKind::InvalidFieldAssignmentTarget {
                                field: field.clone(),
                            },
                            span: *span,
                        });
                    }
                } else if let Some(struct_name) = self.expression_struct_type(object) {
                    self.ensure_struct_field_mutable(&struct_name, field, *span)?;
                }
                Ok(())
            }
            Expr::Index { target, .. } => self.validate_index_assignment_owner(target),
            _ => Ok(()),
        }
    }
}

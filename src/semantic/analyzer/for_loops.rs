use crate::ast::{Block, Expr, ForBinding, Role};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

impl Analyzer {
    pub(super) fn visit_for_range(
        &mut self,
        binding: &ForBinding,
        start: &Expr,
        end: &Expr,
        body: &Block,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if binding.role != Role::Erg {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidArgumentRole {
                    callee: "for".to_owned(),
                    parameter: "range binding must use `erg`".to_owned(),
                },
                span: binding.span,
            });
        }
        let parsed_type = self.for_range_type(binding, start)?;
        let type_name = crate::ast::TypeName::builtin(
            parsed_type.name.clone(),
            parsed_type.arguments.clone(),
            parsed_type.reference_role.clone(),
            parsed_type.span,
        );
        self.visit_expression_with_expected(start, Some(&type_name))?;
        self.visit_expression_with_expected(end, Some(&type_name))?;
        self.enter_scope(body.span);
        self.loop_boundaries.push(self.scopes.len() - 1);
        let binding_type = self.resolve_binding_type(Some(&type_name), start, span)?;
        self.bind(Role::Erg, binding.name.clone(), binding_type, binding.span)?;
        let index = self.binding(&binding.name, binding.span)?;
        self.binding_type_names.insert(index, self.normalize_imported_type_name(&type_name));
        let result = self.visit_block(body);
        self.loop_boundaries.pop();
        self.leave_scope();
        result
    }

    /// The integer type of a range binding: its annotation, or without one
    /// the type of the range's start, which is `Int` for a plain literal.
    fn for_range_type(
        &mut self,
        binding: &ForBinding,
        start: &Expr,
    ) -> Result<crate::ast::TypeName, SemanticError> {
        let declared = match binding.ty.as_deref() {
            Some(name) => Some(name.to_owned()),
            None => {
                self.visit_expression(start)?;
                Some(self.expression_type_name(start).unwrap_or_else(|| "Int".to_owned()))
            }
        };
        let parsed_type = declared
            .as_deref()
            .and_then(|name| super::super::calls::parse_type_name_key(name, binding.span))
            .ok_or_else(|| SemanticError {
                kind: SemanticErrorKind::MalformedTypeName {
                    name: "for range binding requires an integer type".to_owned(),
                },
                span: binding.span,
            })?;
        if parsed_type.name == "Int"
            || matches!(
                crate::ast::primitive_type(&parsed_type.name),
                Some(crate::ast::PrimitiveType::Integer { .. })
            )
        {
            Ok(parsed_type)
        } else {
            Err(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    callee: "for".to_owned(),
                    parameter: "range binding".to_owned(),
                    expected: "integer".to_owned(),
                    found: parsed_type.name,
                },
                span: binding.span,
            })
        }
    }

    pub(super) fn visit_for_array(
        &mut self,
        binding: &ForBinding,
        collection: &Expr,
        body: &Block,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(collection)?;
        let is_array = self.expression_type(collection) == Some(crate::ast::BuiltinType::Array)
            || self.expression_type_name(collection).is_some_and(|name| name.starts_with("Array["));
        if !is_array {
            return Err(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    callee: "for".to_owned(),
                    parameter: "array source".to_owned(),
                    expected: "fixed Array".to_owned(),
                    found: "non-array".to_owned(),
                },
                span,
            });
        }
        let zero =
            Expr::Integer { value: "0".to_owned(), suffix: binding.ty.clone(), span: binding.span };
        self.visit_for_range(binding, &zero, &zero, body, span)
    }
}

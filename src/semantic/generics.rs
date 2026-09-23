use crate::ast::{GenericParam, TypeName, lookup_builtin_type};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ResolvedType {
    GenericParameter(String),
    Concrete(String),
    Applied { name: String, arguments: Vec<ResolvedType> },
}

impl Analyzer {
    pub(super) fn with_generic_scope<T>(
        &mut self,
        parameters: &[GenericParam],
        validate: impl FnOnce(&mut Analyzer) -> Result<T, SemanticError>,
    ) -> Result<T, SemanticError> {
        self.generic_scopes
            .push(parameters.iter().map(|parameter| parameter.name.clone()).collect());
        let result = self.validate_generic_bounds(parameters).and_then(|()| validate(self));
        self.generic_scopes.pop();
        result
    }

    fn validate_generic_bounds(&self, parameters: &[GenericParam]) -> Result<(), SemanticError> {
        for parameter in parameters {
            let Some(bound) = &parameter.bound else { continue };
            if !bound.arguments.is_empty() {
                return Err(arity_error(&bound.name, 0, bound.arguments.len(), bound.span));
            }
            if self.is_generic_parameter(&bound.name) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UnknownTypeParameter { name: bound.name.clone() },
                    span: bound.span,
                });
            }
            // Role declarations and their scope are introduced by Phase 14. Keeping the
            // bound as a validated, unresolved name preserves the generic API now.
        }
        Ok(())
    }

    pub(super) fn validate_type_reference(
        &self,
        type_name: &TypeName,
    ) -> Result<(), SemanticError> {
        self.resolve_type_reference(type_name).map(|_| ())
    }

    fn resolve_type_reference(&self, type_name: &TypeName) -> Result<ResolvedType, SemanticError> {
        let name = type_name.name.as_str();
        if self.is_generic_parameter(name) {
            if !type_name.arguments.is_empty() {
                return Err(arity_error(name, 0, type_name.arguments.len(), type_name.span));
            }
            return Ok(ResolvedType::GenericParameter(name.to_owned()));
        } else if let Some(expected) = self.named_type_arity(name) {
            if expected != type_name.arguments.len() {
                return Err(arity_error(name, expected, type_name.arguments.len(), type_name.span));
            }
        } else if self.generic_scopes.iter().any(|scope| !scope.is_empty()) {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownTypeParameter { name: name.to_owned() },
                span: type_name.span,
            });
        } else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownType { name: name.to_owned() },
                span: type_name.span,
            });
        }

        if type_name.arguments.is_empty() {
            return Ok(ResolvedType::Concrete(name.to_owned()));
        }
        let arguments = type_name
            .arguments
            .iter()
            .map(|argument| self.resolve_type_reference(argument))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ResolvedType::Applied { name: name.to_owned(), arguments })
    }

    pub(super) fn is_generic_parameter(&self, name: &str) -> bool {
        self.generic_scopes.iter().rev().any(|scope| scope.contains(name))
    }

    fn named_type_arity(&self, name: &str) -> Option<usize> {
        if lookup_builtin_type(name).is_some() {
            return Some(0);
        }
        self.struct_types.get(name).map(|definition| definition.generic_parameters.len()).or_else(
            || self.enum_types.get(name).map(|definition| definition.generic_parameters.len()),
        )
    }
}

fn arity_error(name: &str, expected: usize, found: usize, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::GenericArityMismatch { name: name.to_owned(), expected, found },
        span,
    }
}

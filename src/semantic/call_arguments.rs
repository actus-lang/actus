use crate::ast::{Argument, DispatchMode, Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::argument_shapes::{
    argument_span, argument_type_matches, expression_span, is_scalar_name,
};
use super::calls::VerbSignature;
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::{AccessState, OwnershipState};

impl Analyzer {
    pub(super) fn validate_argument_type(
        &self,
        callee: &str,
        parameter: &str,
        expected: &str,
        dispatch: DispatchMode,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        if dispatch == DispatchMode::Dynamic {
            return self.validate_dynamic_argument(expected, expression);
        }
        self.validate_static_argument(callee, parameter, expected, expression)
    }

    fn validate_dynamic_argument(
        &self,
        expected: &str,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let found = self.expression_type_name(expression).unwrap_or_else(|| "unknown".to_owned());
        let type_name = crate::ast::TypeName {
            name: found.clone(),
            arguments: Vec::new(),
            reference_role: None,
            span: expression_span(expression),
        };
        if self.has_performance(expected, &type_name) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::DynamicRoleMismatch { role: expected.to_owned(), found },
            span: expression_span(expression),
        })
    }

    fn validate_static_argument(
        &self,
        callee: &str,
        parameter: &str,
        expected: &str,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let found = self.expression_type_name(expression).unwrap_or_else(|| "unknown".to_owned());
        if argument_type_matches(expected, &found, expression) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::TypeMismatch {
                callee: callee.to_owned(),
                parameter: parameter.to_owned(),
                expected: expected.to_owned(),
                found,
            },
            span: expression_span(expression),
        })
    }

    pub(super) fn bind_arguments(
        &self,
        callee: &str,
        signature: &VerbSignature,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<Vec<usize>, SemanticError> {
        let named = arguments.iter().any(|argument| argument.name.is_some());
        let positional = arguments.iter().any(|argument| argument.name.is_none());
        if named && positional {
            return Err(SemanticError {
                kind: SemanticErrorKind::MixedArgumentModes { callee: callee.to_owned() },
                span,
            });
        }
        if arguments.len() != signature.params.len() {
            return Err(SemanticError {
                kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                span,
            });
        }
        if !named {
            self.validate_positional_call(callee, signature, span)?;
            return Ok((0..arguments.len()).collect());
        }
        self.bind_named_arguments(callee, signature, arguments)
    }

    fn bind_named_arguments(
        &self,
        callee: &str,
        signature: &VerbSignature,
        arguments: &[Argument],
    ) -> Result<Vec<usize>, SemanticError> {
        let mut indices = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let name = argument.name.as_ref().expect("named call argument");
            let Some(index) = signature.params.iter().position(|parameter| parameter.0 == *name)
            else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UnknownParameter {
                        callee: callee.to_owned(),
                        name: name.clone(),
                    },
                    span: argument_span(argument),
                });
            };
            if indices.contains(&index) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateArgument { name: name.clone() },
                    span: argument_span(argument),
                });
            }
            indices.push(index);
        }
        Ok(indices)
    }

    fn validate_positional_call(
        &self,
        callee: &str,
        signature: &VerbSignature,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        for left in 0..signature.params.len() {
            for right in (left + 1)..signature.params.len() {
                let (_, left_role, left_type) = &signature.params[left];
                let (_, right_role, right_type) = &signature.params[right];
                if left_role == right_role && left_type == right_type {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::AmbiguousPositionalCall {
                            callee: callee.to_owned(),
                        },
                        span,
                    });
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate_argument_role(
        &self,
        callee: &str,
        parameter: &str,
        role: &Role,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let valid = match role {
            Role::Dat => true,
            Role::Erg => self.is_active_owner_argument(expression),
            Role::Abs => self.is_borrow_argument(expression),
            Role::Ins => self.is_exclusive_owner(expression),
        };
        if valid {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidArgumentRole {
                callee: callee.to_owned(),
                parameter: parameter.to_owned(),
            },
            span: expression_span(expression),
        })
    }

    pub(super) fn is_owner_argument(&self, expression: &Expr) -> bool {
        let index = match expression {
            Expr::Identifier { name, span } => self.binding(name, *span).ok(),
            Expr::FieldAccess { .. } => self.root_binding_index(expression),
            _ => None,
        };
        let Some(index) = index else { return false };
        let owner = matches!(self.model.bindings[index].role, Role::Erg | Role::Dat | Role::Ins)
            && matches!(self.model.bindings[index].ownership, OwnershipState::Active)
            && matches!(self.model.bindings[index].access, AccessState::Mutable);
        owner
            && (!matches!(expression, Expr::FieldAccess { .. })
                || self.expression_type(expression).is_some())
    }

    fn is_active_owner_argument(&self, expression: &Expr) -> bool {
        let Some(index) = self.root_binding_index(expression) else { return false };
        let role_allowed = matches!(self.model.bindings[index].role, Role::Erg | Role::Dat)
            || (self.model.bindings[index].role == Role::Ins
                && matches!(expression, Expr::FieldAccess { .. })
                && self.expression_type_name(expression).is_some_and(|name| is_scalar_name(&name)));
        role_allowed
            && matches!(self.model.bindings[index].ownership, OwnershipState::Active)
            && matches!(self.model.bindings[index].access, AccessState::Mutable)
            && (!matches!(expression, Expr::FieldAccess { .. })
                || self.expression_type(expression).is_some())
    }

    fn is_borrow_argument(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Identifier { name, span } => {
                let Ok(index) = self.binding(name, *span) else { return false };
                self.model.bindings[index].role == Role::Abs
                    && !matches!(
                        self.model.bindings[index].ownership,
                        OwnershipState::PartiallyMoved { .. }
                            | OwnershipState::Moved
                            | OwnershipState::Dropped
                    )
            }
            Expr::Index { .. } => self.is_readable_owner(expression),
            Expr::Borrow { expression, .. } => self.is_readable_owner(expression),
            _ => false,
        }
    }

    pub(super) fn is_readable_owner(&self, expression: &Expr) -> bool {
        let Some(index) = self.root_binding_index(expression) else { return false };
        matches!(self.model.bindings[index].role, Role::Erg | Role::Abs | Role::Dat | Role::Ins)
            && matches!(
                self.model.bindings[index].ownership,
                OwnershipState::Active | OwnershipState::PartiallyMoved { .. }
            )
            && !self.model.bindings[index].access.is_suspended()
    }

    pub(super) fn is_exclusive_owner(&self, expression: &Expr) -> bool {
        let Some(index) = self.root_binding_index(expression) else { return false };
        matches!(self.model.bindings[index].role, Role::Erg | Role::Dat | Role::Ins)
            && matches!(self.model.bindings[index].ownership, OwnershipState::Active)
            && matches!(self.model.bindings[index].access, AccessState::Mutable)
    }

    pub(super) fn root_binding_index(&self, expression: &Expr) -> Option<usize> {
        let (name, span) = match expression {
            Expr::Identifier { name, span } => (name, span),
            Expr::FieldAccess { object, .. } => return self.root_binding_index(object),
            Expr::Index { target, .. } => return self.root_binding_index(target),
            Expr::Borrow { expression, .. } => return self.root_binding_index(expression),
            _ => return None,
        };
        self.binding(name, *span).ok()
    }
}

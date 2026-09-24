use crate::ast::{
    Argument, BuiltinType, Expr, ExternalVerbDecl, Role, VerbDecl, lookup_builtin_type,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::call_arguments::argument_span;
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::OwnershipState;

#[derive(Clone)]
pub(super) struct VerbSignature {
    pub(super) params: Vec<(String, Role, String)>,
    pub(super) return_type: Option<BuiltinType>,
}

impl VerbDecl {
    pub(super) fn signature(&self) -> VerbSignature {
        VerbSignature {
            params: self
                .params
                .iter()
                .map(|parameter| {
                    (parameter.name.clone(), parameter.role.clone(), parameter.ty.name.clone())
                })
                .collect(),
            return_type: self
                .return_type
                .as_ref()
                .and_then(|type_name| lookup_builtin_type(&type_name.name)),
        }
    }
}

impl ExternalVerbDecl {
    pub(super) fn signature(&self) -> VerbSignature {
        VerbSignature {
            params: self
                .params
                .iter()
                .map(|parameter| {
                    (parameter.name.clone(), parameter.role.clone(), parameter.ty.name.clone())
                })
                .collect(),
            return_type: self
                .return_type
                .as_ref()
                .and_then(|type_name| lookup_builtin_type(&type_name.name)),
        }
    }
}

impl Analyzer {
    pub(super) fn ensure_field_access_readable(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, object_span)) = root_binding(object) else {
            return Ok(());
        };
        let index = self.binding(name, object_span)?;
        let field_path = format_field_path(object, field);
        match &self.model.bindings[index].ownership {
            OwnershipState::Active => Ok(()),
            OwnershipState::PartiallyMoved { fields }
                if field_path.contains('.')
                    && fields.iter().all(|moved| moved.contains('.'))
                    && !fields.iter().any(|moved| paths_overlap(moved, &field_path)) =>
            {
                Ok(())
            }
            OwnershipState::PartiallyMoved { .. } | OwnershipState::Moved => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                span,
            }),
            OwnershipState::Dropped => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterDrop { name: name.clone() },
                span,
            }),
        }
    }

    pub(super) fn visit_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.visit_intrinsic_call(callee, arguments, span)? {
            return Ok(());
        }
        let Some(signature) = self.signatures.get(callee).cloned() else {
            for argument in arguments {
                self.visit_expression(&argument.expression)?;
            }
            return Ok(());
        };
        self.visit_call_with_signature(callee, arguments, span, &signature)
    }

    pub(super) fn visit_call_with_signature(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        let parameter_indices = self.bind_arguments(callee, signature, arguments, span)?;
        for argument in arguments {
            self.visit_expression(&argument.expression)?;
        }
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            let (_, role, _) = &signature.params[parameter_index];
            self.validate_argument_role(
                callee,
                &signature.params[parameter_index].0,
                role,
                &argument.expression,
            )?;
            self.validate_argument_type(
                callee,
                &signature.params[parameter_index].0,
                &signature.params[parameter_index].2,
                &argument.expression,
            )?;
            if *role == Role::Dat {
                self.move_dat_argument(&argument.expression, argument_span(argument))?;
            }
        }
        Ok(())
    }

    pub(super) fn performance_signature(
        &self,
        target_type: &str,
        method: &str,
    ) -> Option<VerbSignature> {
        self.performance_methods.get(&(target_type.to_owned(), method.to_owned())).cloned()
    }

    pub(super) fn performance_role(&self, target_type: &str, method: &str) -> Option<&str> {
        self.performance_roles.get(&(target_type.to_owned(), method.to_owned())).map(String::as_str)
    }

    pub(super) fn mark_reachable_performance(
        &mut self,
        target_type: &str,
        method: &str,
        role_name: &str,
    ) {
        self.reachable_performances.insert(super::model::ReachablePerformance {
            role_name: role_name.to_owned(),
            target_type: target_type.to_owned(),
            method_name: method.to_owned(),
        });
    }

    fn move_dat_argument(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Expr::FieldAccess { object, field, .. } = expression {
            return self.move_struct_field(object, field, span);
        }
        let Expr::Identifier { name, span: identifier_span } = expression else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: "expression".to_owned() },
                span,
            });
        };
        let index = self.binding(name, *identifier_span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: name.clone() },
                span,
            });
        }
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::MoveFrozen {
                    name: name.clone(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        match self.model.bindings[index].ownership.clone() {
            OwnershipState::Active => self.model.bindings[index].ownership = OwnershipState::Moved,
            OwnershipState::PartiallyMoved { .. }
            | OwnershipState::Moved
            | OwnershipState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                    span,
                });
            }
        }
        Ok(())
    }

    fn move_struct_field(
        &mut self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, object_span)) = root_binding(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: field.to_owned() },
                span,
            });
        };
        let index = self.binding(name, object_span)?;
        self.validate_struct_field_move(object, field, span)?;
        let field_path = format_field_path(object, field);
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::FieldBorrowConflict {
                    owner: name.clone(),
                    field: field_path,
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        match self.model.bindings[index].ownership.clone() {
            OwnershipState::Active => {
                self.model.bindings[index].ownership =
                    OwnershipState::PartiallyMoved { fields: vec![field_path] };
                Ok(())
            }
            OwnershipState::PartiallyMoved { fields } => {
                if fields.iter().any(|moved| paths_overlap(moved, &field_path)) {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                        span,
                    });
                }
                let mut updated = fields;
                updated.push(field_path);
                self.model.bindings[index].ownership =
                    OwnershipState::PartiallyMoved { fields: updated };
                Ok(())
            }
            OwnershipState::Moved => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                span,
            }),
            OwnershipState::Dropped => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterDrop { name: name.clone() },
                span,
            }),
        }
    }

    fn validate_struct_field_move(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(struct_name) = self.expression_struct_type(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: field.to_owned() },
                span,
            });
        };
        let Some(struct_field) = self.struct_field(&struct_name, field) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name,
                    field: field.to_owned(),
                },
                span,
            });
        };
        if matches!(struct_field.role, crate::ast::StructFieldRole::Erg) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidDatArgument { name: field.to_owned() },
            span,
        })
    }
}

fn root_binding(expression: &Expr) -> Option<(&String, SourceSpan)> {
    match expression {
        Expr::Identifier { name, span } => Some((name, *span)),
        Expr::FieldAccess { object, .. } => root_binding(object),
        _ => None,
    }
}

fn format_field_path(object: &Expr, field: &str) -> String {
    let prefix = format_expression_path(object);
    if prefix.is_empty() { field.to_owned() } else { format!("{prefix}.{field}") }
}

fn format_expression_path(expression: &Expr) -> String {
    match expression {
        Expr::Identifier { .. } => String::new(),
        Expr::FieldAccess { object, field, .. } => {
            let prefix = format_expression_path(object);
            if prefix.is_empty() { field.clone() } else { format!("{prefix}.{field}") }
        }
        _ => String::new(),
    }
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left.strip_prefix(right).is_some_and(|suffix| suffix.starts_with('.'))
        || right.strip_prefix(left).is_some_and(|suffix| suffix.starts_with('.'))
}

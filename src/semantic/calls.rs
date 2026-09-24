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
        let parameter_indices = self.bind_arguments(callee, &signature, arguments, span)?;
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
        match self.model.bindings[index].ownership {
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
        let Expr::Identifier { name, span: object_span } = object else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: field.to_owned() },
                span,
            });
        };
        let index = self.binding(name, *object_span)?;
        self.validate_struct_field_move(object, field, span)?;
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::FieldBorrowConflict {
                    owner: name.clone(),
                    field: field.to_owned(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        match self.model.bindings[index].ownership {
            OwnershipState::Active => {
                self.model.bindings[index].ownership =
                    OwnershipState::PartiallyMoved { fields: vec![field.to_owned()] };
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

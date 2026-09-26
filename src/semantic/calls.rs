use crate::ast::{
    Argument, BuiltinType, DispatchMode, Expr, ExternalVerbDecl, GenericParam, ReturnAccess, Role,
    VerbDecl, lookup_builtin_type,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::call_arguments::argument_span;
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::OwnershipState;

#[derive(Clone)]
pub(super) struct VerbSignature {
    pub(super) params: Vec<(String, Role, String)>,
    pub(super) dynamic_params: Vec<DispatchMode>,
    pub(super) return_type: Option<BuiltinType>,
    pub(super) return_type_name: Option<crate::ast::TypeName>,
    pub(super) return_access: Option<ReturnAccess>,
    pub(super) generic_parameters: Vec<GenericParam>,
}

impl VerbDecl {
    pub(super) fn signature(&self) -> VerbSignature {
        VerbSignature {
            params: self
                .params
                .iter()
                .map(|parameter| {
                    (
                        parameter.name.clone(),
                        parameter.role.clone(),
                        super::analyzer::canonical_type_name(&parameter.ty),
                    )
                })
                .collect(),
            dynamic_params: self.params.iter().map(|parameter| parameter.dispatch).collect(),
            return_type: self
                .return_type
                .as_ref()
                .and_then(|return_type| lookup_builtin_type(&return_type.ty.name)),
            return_type_name: self.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            return_access: self.return_type.as_ref().map(|return_type| return_type.access),
            generic_parameters: self.generic_parameters.clone(),
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
                    (
                        parameter.name.clone(),
                        parameter.role.clone(),
                        super::analyzer::canonical_type_name(&parameter.ty),
                    )
                })
                .collect(),
            dynamic_params: self.params.iter().map(|parameter| parameter.dispatch).collect(),
            return_type: self
                .return_type
                .as_ref()
                .and_then(|return_type| lookup_builtin_type(&return_type.ty.name)),
            return_type_name: self.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            return_access: self.return_type.as_ref().map(|return_type| return_type.access),
            generic_parameters: self.generic_parameters.clone(),
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
        self.ensure_access_available(index, name, span)?;
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
        if matches!(callee, "Ok" | "Err") && !self.signatures.contains_key(callee) {
            return self.visit_result_constructor(callee, arguments, span);
        }
        if !self.signatures.contains_key(callee)
            && self.visit_intrinsic_call(callee, arguments, span)?
        {
            return Ok(());
        }
        let Some(mut signature) = self.signatures.get(callee).cloned() else {
            for argument in arguments {
                self.visit_expression(&argument.expression)?;
            }
            return Ok(());
        };
        if !signature.generic_parameters.is_empty() {
            signature = self.instantiate_generic_signature(&signature, arguments, span)?;
        }
        self.visit_call_with_signature(callee, arguments, span, &signature)
    }

    fn visit_result_constructor(
        &mut self,
        constructor: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(result_type) = self
            .expected_expression_type
            .clone()
            .filter(|type_name| type_name.name == "Result" && type_name.arguments.len() == 2)
        else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnresolvedResultConstructor {
                    constructor: constructor.to_owned(),
                },
                span,
            });
        };
        if arguments.len() != 1 || arguments[0].name.is_some() {
            return Err(SemanticError {
                kind: SemanticErrorKind::WrongArgumentCount { callee: constructor.to_owned() },
                span,
            });
        }
        let payload_index = usize::from(constructor == "Err");
        let payload_type = &result_type.arguments[payload_index];
        self.visit_expression_with_expected(&arguments[0].expression, Some(payload_type))?;
        let found = self
            .expression_type_name(&arguments[0].expression)
            .unwrap_or_else(|| "unknown".to_owned());
        if found != super::analyzer::canonical_type_name(payload_type) {
            return Err(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    callee: constructor.to_owned(),
                    parameter: "payload".to_owned(),
                    expected: super::analyzer::canonical_type_name(payload_type),
                    found,
                },
                span: argument_span(&arguments[0]),
            });
        }
        self.inferred_expression_types.insert((span.start, span.end), result_type);
        Ok(())
    }

    pub(super) fn visit_call_with_signature(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        let parameter_indices = self.bind_arguments(callee, signature, arguments, span)?;
        for (argument, parameter_index) in arguments.iter().zip(&parameter_indices) {
            let expected =
                parse_type_name_key(&signature.params[*parameter_index].2, argument_span(argument));
            self.visit_expression_with_expected(&argument.expression, expected.as_ref())?;
        }
        for (argument, parameter_index) in arguments.iter().zip(&parameter_indices) {
            self.validate_call_argument(callee, argument, *parameter_index, signature)?;
        }
        self.validate_exclusive_aliases(arguments, &parameter_indices, signature)?;
        for (argument, parameter_index) in arguments.iter().zip(&parameter_indices) {
            if signature.params[*parameter_index].1 == Role::Dat {
                self.move_dat_argument(&argument.expression, argument_span(argument))?;
            }
        }
        self.execute_exclusive_loans(callee, arguments, &parameter_indices, signature)?;
        if let Some(return_type) = &signature.return_type_name {
            self.inferred_expression_types.insert((span.start, span.end), return_type.clone());
        }
        Ok(())
    }

    fn validate_call_argument(
        &self,
        callee: &str,
        argument: &Argument,
        parameter_index: usize,
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        let (_, role, ty) = &signature.params[parameter_index];
        if (*role == Role::Ins && argument.role != Some(Role::Ins))
            || argument.role.as_ref().is_some_and(|actual| actual != role)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidArgumentRole {
                    callee: callee.to_owned(),
                    parameter: signature.params[parameter_index].0.clone(),
                },
                span: argument_span(argument),
            });
        }
        let explicit_owner_view = *role == Role::Abs
            && argument.role == Some(Role::Abs)
            && self.is_readable_owner(&argument.expression);
        if !explicit_owner_view {
            self.validate_argument_role(
                callee,
                &signature.params[parameter_index].0,
                role,
                &argument.expression,
            )?;
        }
        self.validate_argument_type(
            callee,
            &signature.params[parameter_index].0,
            ty,
            signature.dynamic_params[parameter_index],
            &argument.expression,
        )
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
        if matches!(expression, Expr::Call { callee, .. } if matches!(callee.as_str(), "Ok" | "Err"))
        {
            return Ok(());
        }
        let Expr::Identifier { name, span: identifier_span } = expression else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: "expression".to_owned() },
                span,
            });
        };
        let index = self.binding(name, *identifier_span)?;
        self.ensure_access_available(index, name, span)?;
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
        self.ensure_access_available(index, name, span)?;
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

pub(super) fn parse_type_name_key(key: &str, span: SourceSpan) -> Option<crate::ast::TypeName> {
    let Some(open) = key.find('[') else {
        return Some(crate::ast::TypeName { name: key.to_owned(), arguments: Vec::new(), span });
    };
    if !key.ends_with(']') {
        return None;
    }
    let name = key[..open].to_owned();
    let inner = &key[open + 1..key.len() - 1];
    let arguments = split_type_arguments(inner)
        .into_iter()
        .map(|argument| parse_type_name_key(argument, span))
        .collect::<Option<Vec<_>>>()?;
    Some(crate::ast::TypeName { name, arguments, span })
}

fn split_type_arguments(input: &str) -> Vec<&str> {
    let mut depth = 0;
    let mut start = 0;
    let mut parts = Vec::new();
    for (index, character) in input.char_indices() {
        match character {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < input.len() {
        parts.push(input[start..].trim());
    }
    parts
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

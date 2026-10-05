use crate::ast::{Argument, Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::argument_shapes::argument_span;
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::OwnershipState;

mod ownership;
mod parsing;
mod paths;
mod signatures;

pub(super) use parsing::parse_type_name_key;
use paths::{format_field_path, paths_overlap, root_binding};
pub(super) use signatures::VerbSignature;

impl Analyzer {
    pub(super) fn ensure_field_access_readable(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, object_span)) = root_binding(object) else { return Ok(()) };
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
        if self.visit_scalar_copy_call(callee, arguments, span)? {
            return Ok(());
        }
        if let Some(result) = self.visit_special_call(callee, arguments, span)? {
            return result;
        }
        let explicit_type_application = parse_generic_call_arguments(callee, span);
        let lookup_name = explicit_type_application
            .as_ref()
            .map_or(callee, |type_name| type_name.name.as_str())
            .to_owned();
        let Some(mut signature) = self.signatures.get(&lookup_name).cloned() else {
            for argument in arguments {
                self.visit_expression(&argument.expression)?;
            }
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownVerb { name: lookup_name.clone() },
                span,
            });
        };
        if !signature.generic_parameters.is_empty() {
            signature = self.instantiate_generic_signature(
                &lookup_name,
                &signature,
                arguments,
                explicit_type_application.as_ref().map(|type_name| type_name.arguments.as_slice()),
                span,
            )?;
        } else if let Some(type_name) = explicit_type_application {
            return Err(SemanticError {
                kind: SemanticErrorKind::GenericArityMismatch {
                    name: lookup_name.to_owned(),
                    expected: 0,
                    found: type_name.arguments.len(),
                },
                span: type_name.span,
            });
        }
        self.visit_call_with_signature(&lookup_name, arguments, span, &signature)
    }

    fn visit_scalar_copy_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let specialized = callee.starts_with("copy__")
            && arguments.len() == 1
            && arguments[0].name.as_deref() == Some("value");
        let intrinsic = specialized || self.should_use_scalar_copy_intrinsic(callee, arguments);
        if intrinsic {
            self.visit_intrinsic_call("copy", arguments, span)?;
        }
        Ok(intrinsic)
    }

    fn should_use_scalar_copy_intrinsic(&self, callee: &str, arguments: &[Argument]) -> bool {
        if callee != "copy" || arguments.len() != 1 {
            return false;
        }
        if arguments[0].name.as_deref() != Some("value") {
            return false;
        }
        if !self.local_signatures.contains(callee) {
            return true;
        }
        self.signatures
            .get(callee)
            .is_none_or(|signature| signature.params.len() != 1 || signature.params[0].0 != "value")
    }

    fn visit_special_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<Option<Result<(), SemanticError>>, SemanticError> {
        if let Some(type_name) = arena_constructor_type(callee, span) {
            self.validate_type_reference(&type_name)?;
            if !arguments.is_empty() {
                return Ok(Some(Err(SemanticError {
                    kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                    span,
                })));
            }
            self.inferred_expression_types.insert((span.start, span.end), type_name);
            return Ok(Some(Ok(())));
        }
        if let Some(type_name) = array_constructor_type(callee, span) {
            self.validate_type_reference(&type_name)?;
            if !arguments.is_empty() {
                return Ok(Some(Err(SemanticError {
                    kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                    span,
                })));
            }
            self.inferred_expression_types.insert((span.start, span.end), type_name);
            return Ok(Some(Ok(())));
        }
        if matches!(callee, "Ok" | "Err") && !self.signatures.contains_key(callee) {
            return Ok(Some(self.visit_result_constructor(callee, arguments, span)));
        }
        if !self.signatures.contains_key(callee)
            && self.visit_intrinsic_call(callee, arguments, span)?
        {
            return Ok(Some(Ok(())));
        }
        Ok(None)
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
        let payload_type = &result_type.arguments[usize::from(constructor == "Err")];
        self.validate_result_payload(constructor, &arguments[0], payload_type)?;
        self.inferred_expression_types.insert((span.start, span.end), result_type);
        Ok(())
    }

    fn validate_result_payload(
        &mut self,
        constructor: &str,
        argument: &Argument,
        payload_type: &crate::ast::TypeName,
    ) -> Result<(), SemanticError> {
        self.visit_expression_with_expected(&argument.expression, Some(payload_type))?;
        let found =
            self.expression_type_name(&argument.expression).unwrap_or_else(|| "unknown".to_owned());
        if found != super::analyzer::canonical_type_name(payload_type) {
            return Err(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    callee: constructor.to_owned(),
                    parameter: "payload".to_owned(),
                    expected: super::analyzer::canonical_type_name(payload_type),
                    found,
                },
                span: argument_span(argument),
            });
        }
        if self.enum_payload_owns_value(payload_type) {
            self.initialize_owner(&argument.expression, argument_span(argument))?;
        }
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
        self.validate_call_role(callee, argument, parameter_index, signature, role)?;
        self.validate_argument_type(
            callee,
            &signature.params[parameter_index].0,
            ty,
            signature.dynamic_params[parameter_index],
            &argument.expression,
        )
    }

    fn validate_call_role(
        &self,
        callee: &str,
        argument: &Argument,
        parameter_index: usize,
        signature: &VerbSignature,
        role: &Role,
    ) -> Result<(), SemanticError> {
        if !call_role_matches(role, argument) {
            return Err(invalid_call_role(callee, parameter_index, argument, signature));
        }
        self.validate_explicit_owner_view(callee, argument, parameter_index, signature, role)
    }

    fn validate_explicit_owner_view(
        &self,
        callee: &str,
        argument: &Argument,
        parameter_index: usize,
        signature: &VerbSignature,
        role: &Role,
    ) -> Result<(), SemanticError> {
        if *role == Role::Abs
            && argument.role == Some(Role::Abs)
            && self.is_readable_owner(&argument.expression)
        {
            return Ok(());
        }
        self.validate_argument_role(
            callee,
            &signature.params[parameter_index].0,
            role,
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
}

fn call_role_matches(role: &Role, argument: &Argument) -> bool {
    !(*role == Role::Ins && argument.role != Some(Role::Ins))
        && !argument.role.as_ref().is_some_and(|actual| actual != role)
}

fn invalid_call_role(
    callee: &str,
    parameter_index: usize,
    argument: &Argument,
    signature: &VerbSignature,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidArgumentRole {
            callee: callee.to_owned(),
            parameter: signature.params[parameter_index].0.clone(),
        },
        span: argument_span(argument),
    }
}

fn arena_constructor_type(callee: &str, span: SourceSpan) -> Option<crate::ast::TypeName> {
    callee.starts_with("Arena[").then(|| parse_type_name_key(callee, span)).flatten()
}

fn array_constructor_type(callee: &str, span: SourceSpan) -> Option<crate::ast::TypeName> {
    callee.starts_with("Array[").then(|| parse_type_name_key(callee, span)).flatten()
}

fn parse_generic_call_arguments(callee: &str, span: SourceSpan) -> Option<crate::ast::TypeName> {
    let type_name = parse_type_name_key(callee, span)?;
    (!type_name.arguments.is_empty()).then_some(type_name)
}

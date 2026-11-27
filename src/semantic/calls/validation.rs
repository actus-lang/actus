use crate::ast::{Argument, Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::{ArgumentRoleFact, ArgumentRoleSource};
use super::shapes::argument_span;
use super::state::OwnershipState;
use super::{VerbSignature, shapes};

use super::paths::{format_field_path, paths_overlap, root_binding};

impl Analyzer {
    pub(in crate::semantic) fn ensure_field_access_readable(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, object_span)) = root_binding(object) else { return Ok(()) };
        let index = self.binding(name, object_span)?;
        self.ensure_arena_alive(index, name, span)?;
        let field_path = format_field_path(object, field);
        match &self.model.bindings[index].ownership {
            OwnershipState::Active => Ok(()),
            // Moves are tracked per field path (ADR-0016, ADR-0046 2.2): only a
            // path that overlaps a moved one is unusable; disjoint fields stay.
            OwnershipState::PartiallyMoved { fields }
                if !fields.iter().any(|moved| paths_overlap(moved, &field_path)) =>
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

    pub(super) fn visit_special_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        resolved_type: Option<&crate::ast::TypeName>,
        generic_arguments: &[crate::ast::TypeName],
        span: SourceSpan,
    ) -> Result<Option<Result<(), SemanticError>>, SemanticError> {
        if is_arena_constructor(callee) {
            let type_name = resolved_type.expect(
                "internal compiler error: arena constructor reached semantic analysis unresolved",
            );
            self.validate_type_reference(type_name)?;
            if !arguments.is_empty() {
                return Ok(Some(Err(SemanticError {
                    kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                    span,
                })));
            }
            self.inferred_expression_types
                .insert(self.expression_cache_key(span), type_name.clone());
            return Ok(Some(Ok(())));
        }
        if is_array_constructor(callee) {
            let mut normalized_type_name = resolved_type.cloned().expect(
                "internal compiler error: array constructor reached semantic analysis unresolved",
            );
            self.normalize_array_capacities(&mut normalized_type_name)?;
            if !arguments.is_empty() {
                return Ok(Some(Err(SemanticError {
                    kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                    span,
                })));
            }
            self.inferred_expression_types
                .insert(self.expression_cache_key(span), normalized_type_name);
            return Ok(Some(Ok(())));
        }
        if matches!(callee, "Ok" | "Err") && !self.signatures.contains_key(callee) {
            return Ok(Some(self.visit_result_constructor(callee, arguments, span)));
        }
        if !self.signatures.contains_key(callee)
            && self.visit_intrinsic_call_with_types(callee, arguments, span, generic_arguments)?
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
        self.inferred_expression_types.insert(self.expression_cache_key(span), result_type);
        Ok(())
    }

    fn validate_result_payload(
        &mut self,
        constructor: &str,
        argument: &Argument,
        payload_type: &crate::ast::TypeName,
    ) -> Result<(), SemanticError> {
        self.visit_expression_with_expected(&argument.expression, Some(payload_type))?;
        let found = self
            .expression_type_name(&argument.expression)
            .map(|name| self.normalize_imported_type_key(&name))
            .unwrap_or_else(|| "unknown".to_owned());
        let expected =
            self.normalize_imported_type_key(&super::analyzer::canonical_type_name(payload_type));
        if found != expected {
            return Err(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    callee: constructor.to_owned(),
                    parameter: "payload".to_owned(),
                    expected,
                    found,
                },
                span: argument_span(argument),
            });
        }
        self.store_enum_payload(payload_type, &argument.expression, argument_span(argument))
    }

    /// Moves every argument that a `dat` or `erg` parameter takes over.
    fn move_transferred_arguments(
        &mut self,
        arguments: &[Argument],
        parameter_indices: &[usize],
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            match signature.params[*parameter_index].1 {
                Role::Dat => {
                    self.move_dat_argument(&argument.expression, argument_span(argument))?;
                }
                // An `erg` parameter is a new owner inside the callee, which
                // cleans it up at its scope end (ADR-0016, ADR-0059). A struct,
                // an enum or an argument that holds a resource therefore
                // leaves the caller as for `dat` (ADR-0124); a scalar is read
                // and stays usable.
                Role::Erg
                    if self.argument_moves_source(
                        &argument.expression,
                        &signature.params[*parameter_index].2,
                    ) =>
                {
                    self.consume_owned_source(&argument.expression, argument_span(argument))?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(in crate::semantic) fn visit_call_with_signature(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        self.reject_void_call_value(callee, signature, span)?;
        self.validate_effects(callee, signature, span)?;
        let discarded = self.discarded_expression;
        self.discarded_expression = false;
        let parameter_indices = self.bind_arguments(callee, signature, arguments, span)?;
        let resolved_arguments =
            self.resolve_argument_roles(callee, arguments, &parameter_indices, signature, span);
        for (argument, parameter_index) in resolved_arguments.iter().zip(&parameter_indices) {
            let expected =
                Some(self.normalize_imported_type_name(&signature.params[*parameter_index].2));
            self.visit_expression_with_expected(&argument.expression, expected.as_ref())?;
        }
        for (argument, parameter_index) in resolved_arguments.iter().zip(&parameter_indices) {
            self.validate_call_argument(callee, argument, *parameter_index, signature)?;
        }
        self.validate_exclusive_aliases(&resolved_arguments, &parameter_indices, signature)?;
        self.move_transferred_arguments(&resolved_arguments, &parameter_indices, signature)?;
        self.execute_exclusive_loans(callee, &resolved_arguments, &parameter_indices, signature)?;
        if let Some(return_type) = &signature.return_type_name {
            let return_type = self.normalize_imported_type_name(return_type);
            self.inferred_expression_types.insert(self.expression_cache_key(span), return_type);
        }
        self.discarded_expression = discarded;
        if discarded
            && !self.expression_value_consumed
            && let Some(reason) = &signature.must_use
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::MustUseIgnored {
                    callee: callee.to_owned(),
                    reason: reason.clone(),
                },
                span,
            });
        }
        Ok(())
    }

    fn resolve_argument_roles(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        parameter_indices: &[usize],
        signature: &VerbSignature,
        call_span: SourceSpan,
    ) -> Vec<Argument> {
        arguments
            .iter()
            .zip(parameter_indices)
            .map(|(argument, parameter_index)| {
                let expected_role = &signature.params[*parameter_index].1;
                if let Some(role) = &argument.role {
                    self.record_argument_role(
                        callee,
                        &signature.params[*parameter_index].0,
                        role.clone(),
                        ArgumentRoleSource::Explicit,
                        call_span,
                        argument,
                    );
                    return argument.clone();
                }
                let inferred = self.infer_argument_role(
                    callee,
                    argument,
                    *parameter_index,
                    expected_role,
                    signature,
                );
                let Some(role) = inferred else { return argument.clone() };
                let mut resolved = argument.clone();
                resolved.role = Some(role.clone());
                resolved.role_resolution = crate::ast::ArgumentRoleResolution::Inferred;
                self.record_argument_role(
                    callee,
                    &signature.params[*parameter_index].0,
                    role,
                    ArgumentRoleSource::Inferred,
                    call_span,
                    &resolved,
                );
                resolved
            })
            .collect()
    }

    fn infer_argument_role(
        &self,
        callee: &str,
        argument: &Argument,
        parameter_index: usize,
        expected_role: &Role,
        signature: &VerbSignature,
    ) -> Option<Role> {
        if !self.local_signatures.contains(callee)
            || signature.external
            || signature
                .dynamic_params
                .get(parameter_index)
                .is_some_and(|mode| *mode != crate::ast::DispatchMode::Static)
        {
            return None;
        }
        match expected_role {
            Role::Abs if self.infer_abs_argument(argument) => Some(Role::Abs),
            Role::Ins if self.infer_ins_argument(argument) => Some(Role::Ins),
            _ => None,
        }
    }

    fn infer_abs_argument(&self, argument: &Argument) -> bool {
        matches!(argument.expression, crate::ast::Expr::Identifier { .. })
            && self.is_borrow_argument(&argument.expression)
    }

    fn infer_ins_argument(&self, argument: &Argument) -> bool {
        let Some(root) = self.root_binding_index(&argument.expression) else { return false };
        self.model.bindings[root].role == Role::Ins
            && matches!(argument.expression, crate::ast::Expr::Identifier { .. })
            && self
                .expression_type_name(&argument.expression)
                .is_some_and(|name| shapes::is_scalar_name(&name))
            && self.is_exclusive_owner(&argument.expression)
    }

    fn record_argument_role(
        &mut self,
        callee: &str,
        parameter: &str,
        role: Role,
        source: ArgumentRoleSource,
        call_span: SourceSpan,
        argument: &Argument,
    ) {
        self.model.argument_roles.push(ArgumentRoleFact {
            callee: callee.to_owned(),
            parameter: parameter.to_owned(),
            role,
            source,
            call_span,
            argument_span: argument_span(argument),
        });
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
            &super::super::analyzer::canonical_type_name(ty),
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

    pub(in crate::semantic) fn performance_signature(
        &self,
        target_type: &str,
        method: &str,
    ) -> Option<VerbSignature> {
        self.performance_methods.get(&(target_type.to_owned(), method.to_owned())).cloned()
    }

    pub(in crate::semantic) fn performance_role(
        &self,
        target_type: &str,
        method: &str,
    ) -> Option<&str> {
        self.performance_roles.get(&(target_type.to_owned(), method.to_owned())).map(String::as_str)
    }

    pub(in crate::semantic) fn mark_reachable_performance(
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

fn is_arena_constructor(callee: &str) -> bool {
    callee.starts_with("Arena[")
}

fn is_array_constructor(callee: &str) -> bool {
    callee.starts_with("Array[")
}

use crate::ast::{GenericParam, GenericParamKind, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::type_substitution::TypeSubstitution;

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
        let previous = parameters
            .iter()
            .map(|parameter| {
                (parameter.name.clone(), self.generic_bounds.get(&parameter.name).cloned())
            })
            .collect::<Vec<_>>();
        for parameter in parameters {
            let bounds = parameter_bounds(parameter)
                .into_iter()
                .map(canonical_type_name)
                .collect::<Vec<_>>();
            self.generic_bounds.insert(parameter.name.clone(), bounds);
        }
        self.generic_scopes
            .push(parameters.iter().map(|parameter| parameter.name.clone()).collect());
        self.const_generic_scopes.push(
            parameters
                .iter()
                .filter(|parameter| matches!(parameter.kind, GenericParamKind::Const { .. }))
                .map(|parameter| parameter.name.clone())
                .collect(),
        );
        let result = self.validate_generic_bounds(parameters).and_then(|()| validate(self));
        self.generic_scopes.pop();
        self.const_generic_scopes.pop();
        for (name, bound) in previous {
            match bound {
                Some(bound) => {
                    self.generic_bounds.insert(name, bound);
                }
                None => {
                    self.generic_bounds.remove(&name);
                }
            }
        }
        result
    }

    fn validate_generic_bounds(&self, parameters: &[GenericParam]) -> Result<(), SemanticError> {
        for parameter in parameters {
            if let GenericParamKind::Const { domain } = &parameter.kind {
                if domain.name != "Usize" || !domain.arguments.is_empty() {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::GenericConstraintMismatch {
                            parameter: parameter.name.clone(),
                            constraint: "Usize".to_owned(),
                            argument: domain.name.clone(),
                        },
                        span: domain.span,
                    });
                }
                continue;
            }
            for bound in parameter_bounds(parameter) {
                if !bound.arguments.is_empty() {
                    return Err(arity_error(&bound.name, 0, bound.arguments.len(), bound.span));
                }
                if self.is_generic_parameter(&bound.name) {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::UnknownTypeParameter { name: bound.name.clone() },
                        span: bound.span,
                    });
                }
                if bound.name != "Numeric" && !self.role_types.contains_key(&bound.name) {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::UnknownRole { name: bound.name.clone() },
                        span: bound.span,
                    });
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate_type_reference(
        &mut self,
        type_name: &TypeName,
    ) -> Result<(), SemanticError> {
        self.resolve_type_reference(type_name)?;
        self.record_generic_instances(type_name);
        Ok(())
    }

    fn resolve_type_reference(&self, type_name: &TypeName) -> Result<ResolvedType, SemanticError> {
        let name = type_name.name.as_str();
        if name == "Arena" {
            return self.resolve_arena_type(type_name);
        }
        if name == "Array" {
            return self.resolve_array_type(type_name);
        }
        if let Some(generic) = self.resolve_generic_parameter(type_name)? {
            return Ok(generic);
        }
        self.validate_named_type(type_name)?;

        if type_name.arguments.is_empty() {
            return Ok(ResolvedType::Concrete(name.to_owned()));
        }
        let parameters = self.named_type_parameters(name);
        if let Some(parameters) = &parameters {
            TypeSubstitution::for_type(name, parameters, &type_name.arguments, type_name.span)?;
            self.validate_type_arguments(parameters, &type_name.arguments)?;
        }
        let arguments = type_name
            .arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                if parameters.as_ref().and_then(|parameters| parameters.get(index)).is_some_and(
                    |parameter| matches!(parameter.kind, GenericParamKind::Const { .. }),
                ) {
                    Ok(ResolvedType::Concrete(argument.name.clone()))
                } else {
                    self.resolve_type_reference(argument)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ResolvedType::Applied { name: name.to_owned(), arguments })
    }

    fn resolve_arena_type(&self, type_name: &TypeName) -> Result<ResolvedType, SemanticError> {
        self.validate_arena_type(type_name)?;
        Ok(ResolvedType::Applied {
            name: type_name.name.clone(),
            arguments: vec![ResolvedType::Concrete(type_name.arguments[0].name.clone())],
        })
    }

    fn resolve_array_type(&self, type_name: &TypeName) -> Result<ResolvedType, SemanticError> {
        if type_name.arguments.len() != 2 {
            return Err(arity_error("Array", 2, type_name.arguments.len(), type_name.span));
        }
        let element = &type_name.arguments[0];
        let capacity = &type_name.arguments[1];
        let parsed_capacity = capacity.name.parse::<usize>().ok();
        let const_parameter = self.is_const_generic_parameter(&capacity.name);
        if capacity.reference_role.is_some()
            || !capacity.arguments.is_empty()
            || (!const_parameter && parsed_capacity.is_none())
            || parsed_capacity == Some(0)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidArrayCapacity { capacity: capacity.name.clone() },
                span: capacity.span,
            });
        }
        let resolved_element = self.resolve_type_reference(element)?;
        Ok(ResolvedType::Applied {
            name: "Array".to_owned(),
            arguments: vec![resolved_element, ResolvedType::Concrete(capacity.name.clone())],
        })
    }

    fn resolve_generic_parameter(
        &self,
        type_name: &TypeName,
    ) -> Result<Option<ResolvedType>, SemanticError> {
        if !self.is_generic_parameter(&type_name.name) {
            return Ok(None);
        }
        if !type_name.arguments.is_empty() {
            return Err(arity_error(&type_name.name, 0, type_name.arguments.len(), type_name.span));
        }
        Ok(Some(ResolvedType::GenericParameter(type_name.name.clone())))
    }

    fn validate_named_type(&self, type_name: &TypeName) -> Result<(), SemanticError> {
        let name = type_name.name.as_str();
        if let Some(expected) = self.named_type_arity(name) {
            if expected != type_name.arguments.len() {
                return Err(arity_error(name, expected, type_name.arguments.len(), type_name.span));
            }
            return Ok(());
        }
        let kind = if self.generic_scopes.iter().any(|scope| !scope.is_empty()) {
            SemanticErrorKind::UnknownTypeParameter { name: name.to_owned() }
        } else {
            SemanticErrorKind::UnknownType { name: name.to_owned() }
        };
        Err(SemanticError { kind, span: type_name.span })
    }

    pub(super) fn is_generic_parameter(&self, name: &str) -> bool {
        self.generic_scopes.iter().rev().any(|scope| scope.contains(name))
    }

    pub(super) fn is_const_generic_parameter(&self, name: &str) -> bool {
        self.const_generic_scopes.iter().rev().any(|scope| scope.contains(name))
    }

    fn named_type_arity(&self, name: &str) -> Option<usize> {
        if self.type_registry.is_known(name) {
            return Some(0);
        }
        self.struct_types
            .get(name)
            .map(|definition| definition.generic_parameters.len())
            .or_else(|| {
                self.enum_types.get(name).map(|definition| definition.generic_parameters.len())
            })
            .or_else(|| (name == "Arena").then_some(1))
            .or_else(|| self.pack_types.contains_key(name).then_some(0))
    }

    fn validate_arena_type(&self, type_name: &TypeName) -> Result<(), SemanticError> {
        let Some(capacity) = type_name.arguments.first() else {
            return Err(arena_capacity_error(type_name.span, "missing"));
        };
        if type_name.arguments.len() != 1
            || capacity.name.parse::<usize>().is_err()
            || capacity.name == "0"
            || !capacity.arguments.is_empty()
        {
            return Err(arena_capacity_error(capacity.span, &capacity.name));
        }
        Ok(())
    }

    fn named_type_parameters(&self, name: &str) -> Option<Vec<GenericParam>> {
        self.struct_types.get(name).map(|definition| definition.generic_parameters.clone()).or_else(
            || self.enum_types.get(name).map(|definition| definition.generic_parameters.clone()),
        )
    }

    fn record_generic_instances(&mut self, type_name: &TypeName) {
        for argument in &type_name.arguments {
            self.record_generic_instances(argument);
        }
        if type_name.arguments.is_empty()
            || self.is_generic_parameter(&type_name.name)
            || type_name.arguments.iter().any(|argument| self.contains_generic_parameter(argument))
        {
            return;
        }
        let Some(parameters) = self.named_type_parameters(&type_name.name) else { return };
        if parameters.is_empty() {
            return;
        }
        let canonical_key = canonical_type_name(type_name);
        self.generic_instances.insert(super::model::GenericInstance {
            name: type_name.name.clone(),
            arguments: type_name.arguments.clone(),
            canonical_key,
            caller: None,
            call_span: type_name.span,
        });
    }

    fn contains_generic_parameter(&self, type_name: &TypeName) -> bool {
        self.is_generic_parameter(&type_name.name)
            || type_name.arguments.iter().any(|argument| self.contains_generic_parameter(argument))
    }

    pub(super) fn validate_type_arguments(
        &self,
        parameters: &[GenericParam],
        arguments: &[TypeName],
    ) -> Result<(), SemanticError> {
        for (parameter, argument) in parameters.iter().zip(arguments) {
            if matches!(parameter.kind, GenericParamKind::Const { .. }) {
                let valid = argument.reference_role.is_none()
                    && argument.arguments.is_empty()
                    && (argument.name.parse::<usize>().is_ok_and(|value| value > 0)
                        || self.is_const_generic_parameter(&argument.name));
                if !valid {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::GenericConstraintMismatch {
                            parameter: parameter.name.clone(),
                            constraint: "Usize".to_owned(),
                            argument: canonical_type_name(argument),
                        },
                        span: argument.span,
                    });
                }
                continue;
            }
            for bound in parameter_bounds(parameter) {
                if bound.name == "Numeric"
                    && !self.is_generic_parameter(&argument.name)
                    && !is_numeric_type(argument)
                {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::GenericConstraintMismatch {
                            parameter: parameter.name.clone(),
                            constraint: bound.name.clone(),
                            argument: canonical_type_name(argument),
                        },
                        span: argument.span,
                    });
                }
                if bound.name != "Numeric"
                    && !self.is_generic_parameter(&argument.name)
                    && !self.has_performance(&bound.name, argument)
                {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::GenericConstraintMismatch {
                            parameter: parameter.name.clone(),
                            constraint: bound.name.clone(),
                            argument: canonical_type_name(argument),
                        },
                        span: argument.span,
                    });
                }
            }
        }
        Ok(())
    }
}

impl Analyzer {
    pub(super) fn has_performance(&self, role: &str, target: &TypeName) -> bool {
        self.performances.contains(&(role.to_owned(), canonical_type_name(target)))
    }
}

fn parameter_bounds(parameter: &GenericParam) -> Vec<&TypeName> {
    if parameter.bounds.is_empty() {
        parameter.bound.iter().collect()
    } else {
        parameter.bounds.iter().collect()
    }
}

fn is_numeric_type(type_name: &TypeName) -> bool {
    matches!(
        type_name.name.as_str(),
        "Int"
            | "I8"
            | "I16"
            | "I32"
            | "I64"
            | "U8"
            | "U16"
            | "U32"
            | "U64"
            | "Usize"
            | "F32"
            | "F64"
    ) && type_name.arguments.is_empty()
}

fn canonical_type_name(type_name: &TypeName) -> String {
    let role = type_name
        .reference_role
        .as_ref()
        .map(|role| match role {
            crate::ast::Role::Abs => "abs ",
            crate::ast::Role::Ins => "ins ",
            crate::ast::Role::Erg => "erg ",
            crate::ast::Role::Dat => "dat ",
        })
        .unwrap_or("");
    if type_name.arguments.is_empty() {
        return format!("{role}{}", type_name.name);
    }
    format!(
        "{role}{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

fn arena_capacity_error(span: SourceSpan, capacity: &str) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidArenaCapacity { capacity: capacity.to_owned() },
        span,
    }
}

fn arity_error(name: &str, expected: usize, found: usize, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::GenericArityMismatch { name: name.to_owned(), expected, found },
        span,
    }
}

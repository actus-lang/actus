use std::collections::HashMap;

use crate::ast::{Argument, GenericParam, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::argument_shapes::argument_span;
use super::calls::{VerbSignature, parse_type_name_key};
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn instantiate_generic_signature(
        &mut self,
        name: &str,
        signature: &VerbSignature,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<VerbSignature, SemanticError> {
        let bindings = self.infer_generic_bindings(signature, arguments);
        self.record_generic_call_instance(name, signature, &bindings);
        let specialized = self.specialize_signature(signature, bindings, span)?;
        self.record_specialized_signature_instances(&specialized, span)?;
        Ok(specialized)
    }

    fn record_generic_call_instance(
        &mut self,
        name: &str,
        signature: &VerbSignature,
        bindings: &HashMap<String, TypeName>,
    ) {
        let arguments = signature
            .generic_parameters
            .iter()
            .filter_map(|parameter| bindings.get(&parameter.name).cloned())
            .collect::<Vec<_>>();
        if arguments.len() != signature.generic_parameters.len() {
            return;
        }
        let canonical_key = format!(
            "{name}[{}]",
            arguments
                .iter()
                .map(super::analyzer::canonical_type_name)
                .collect::<Vec<_>>()
                .join(",")
        );
        self.generic_instances.insert(super::model::GenericInstance {
            name: name.to_owned(),
            arguments,
            canonical_key,
        });
    }

    fn infer_generic_bindings(
        &self,
        signature: &VerbSignature,
        arguments: &[Argument],
    ) -> HashMap<String, TypeName> {
        let mut bindings = HashMap::new();
        for (index, (_, _, parameter_type)) in signature.params.iter().enumerate() {
            let Some(argument) = arguments.get(index) else { continue };
            let Some(found) = self
                .expression_type_name(&argument.expression)
                .and_then(|name| parse_type_name_key(&name, argument_span(argument)))
            else {
                continue;
            };
            let Some(pattern) = parse_type_name_key(parameter_type, argument_span(argument)) else {
                continue;
            };
            unify_generic_type(&pattern, &found, &signature.generic_parameters, &mut bindings);
        }
        if let (Some(expected), Some(return_type)) =
            (&self.expected_expression_type, &signature.return_type_name)
        {
            unify_generic_type(return_type, expected, &signature.generic_parameters, &mut bindings);
        }
        bindings
    }

    fn record_specialized_signature_instances(
        &mut self,
        signature: &VerbSignature,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        for (_, _, parameter_type) in &signature.params {
            let type_name = parse_type_name_key(parameter_type, span)
                .ok_or_else(|| malformed_type_name(parameter_type, span))?;
            self.validate_type_reference(&type_name)?;
        }
        if let Some(return_type) = &signature.return_type_name {
            self.validate_type_reference(return_type)?;
        }
        Ok(())
    }

    fn specialize_signature(
        &self,
        signature: &VerbSignature,
        bindings: HashMap<String, TypeName>,
        span: SourceSpan,
    ) -> Result<VerbSignature, SemanticError> {
        for parameter in &signature.generic_parameters {
            if !bindings.contains_key(&parameter.name) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UnknownTypeParameter { name: parameter.name.clone() },
                    span,
                });
            }
        }
        let mut params = Vec::with_capacity(signature.params.len());
        for (name, role, ty) in &signature.params {
            let parsed =
                parse_type_name_key(ty, span).ok_or_else(|| malformed_type_name(ty, span))?;
            params.push((
                name.clone(),
                role.clone(),
                super::analyzer::canonical_type_name(&substitute_type(&parsed, &bindings)),
            ));
        }
        Ok(VerbSignature {
            params,
            dynamic_params: signature.dynamic_params.clone(),
            return_type: signature.return_type,
            return_type_name: signature
                .return_type_name
                .as_ref()
                .map(|return_type| substitute_type(return_type, &bindings)),
            return_access: signature.return_access,
            generic_parameters: Vec::new(),
        })
    }
}

fn malformed_type_name(name: &str, span: SourceSpan) -> SemanticError {
    SemanticError { kind: SemanticErrorKind::MalformedTypeName { name: name.to_owned() }, span }
}

fn unify_generic_type(
    pattern: &TypeName,
    concrete: &TypeName,
    parameters: &[GenericParam],
    bindings: &mut HashMap<String, TypeName>,
) {
    if parameters.iter().any(|parameter| parameter.name == pattern.name) {
        bindings.entry(pattern.name.clone()).or_insert_with(|| concrete.clone());
        return;
    }
    if pattern.name != concrete.name || pattern.arguments.len() != concrete.arguments.len() {
        return;
    }
    for (nested_pattern, nested_concrete) in pattern.arguments.iter().zip(&concrete.arguments) {
        unify_generic_type(nested_pattern, nested_concrete, parameters, bindings);
    }
}

fn substitute_type(type_name: &TypeName, bindings: &HashMap<String, TypeName>) -> TypeName {
    if let Some(concrete) = bindings.get(&type_name.name) {
        return concrete.clone();
    }
    TypeName {
        name: type_name.name.clone(),
        arguments: type_name
            .arguments
            .iter()
            .map(|argument| substitute_type(argument, bindings))
            .collect(),
        reference_role: type_name.reference_role.clone(),
        span: type_name.span,
    }
}

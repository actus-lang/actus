use std::collections::HashMap;

use crate::ast::{Argument, GenericParam, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::call_arguments::argument_span;
use super::calls::{VerbSignature, parse_type_name_key};
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn instantiate_generic_signature(
        &self,
        signature: &VerbSignature,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<VerbSignature, SemanticError> {
        let bindings = self.infer_generic_bindings(signature, arguments);
        self.specialize_signature(signature, bindings, span)
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
        let params = signature
            .params
            .iter()
            .map(|(name, role, ty)| {
                let parsed = parse_type_name_key(ty, span).unwrap_or_else(|| TypeName {
                    name: ty.clone(),
                    arguments: Vec::new(),
                    span,
                });
                (
                    name.clone(),
                    role.clone(),
                    super::analyzer::canonical_type_name(&substitute_type(&parsed, &bindings)),
                )
            })
            .collect();
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
        span: type_name.span,
    }
}

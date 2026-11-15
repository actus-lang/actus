use std::collections::HashMap;

use crate::ast::{Argument, GenericParam, TypeIdentity, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use crate::semantic::calls::VerbSignature;
use crate::semantic::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(in crate::semantic) fn instantiate_generic_signature(
        &mut self,
        name: &str,
        signature: &VerbSignature,
        arguments: &[Argument],
        explicit_arguments: Option<&[TypeName]>,
        span: SourceSpan,
    ) -> Result<VerbSignature, SemanticError> {
        let bindings = if let Some(explicit_arguments) = explicit_arguments {
            self.validate_type_arguments(&signature.generic_parameters, explicit_arguments)?;
            signature
                .generic_parameters
                .iter()
                .zip(explicit_arguments)
                .map(|(parameter, argument)| {
                    (parameter.name.clone(), self.normalize_imported_type_name(argument))
                })
                .collect()
        } else {
            self.infer_generic_bindings(signature, arguments)
        };
        self.record_generic_call_instance(name, signature, &bindings, span);
        let specialized = self.specialize_signature(signature, bindings, span)?;
        self.record_specialized_signature_instances(&specialized)?;
        Ok(specialized)
    }

    fn record_generic_call_instance(
        &mut self,
        name: &str,
        signature: &VerbSignature,
        bindings: &HashMap<String, TypeName>,
        span: SourceSpan,
    ) {
        let arguments = signature
            .generic_parameters
            .iter()
            .filter_map(|parameter| bindings.get(&parameter.name).cloned())
            .collect::<Vec<_>>();
        if arguments.len() != signature.generic_parameters.len() {
            return;
        }
        let identity = TypeIdentity::from_verb_application(name, &arguments);
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
            identity,
            canonical_key,
            caller: self.current_generic_owner.clone(),
            call_span: span,
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
            let Some(found) = self.resolved_type_name(&argument.expression) else {
                continue;
            };
            let pattern = self.normalize_imported_type_name(parameter_type);
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
    ) -> Result<(), SemanticError> {
        for (_, _, parameter_type) in &signature.params {
            self.validate_type_reference(parameter_type)?;
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
            let parsed = self.normalize_imported_type_name(ty);
            params.push((name.clone(), role.clone(), substitute_type(&parsed, &bindings)));
        }
        Ok(VerbSignature {
            params,
            dynamic_params: signature.dynamic_params.clone(),
            return_type: signature.return_type,
            return_type_name: signature.return_type_name.as_ref().map(|return_type| {
                let return_type = self.normalize_imported_type_name(return_type);
                substitute_type(&return_type, &bindings)
            }),
            return_access: signature.return_access,
            generic_parameters: Vec::new(),
            external: signature.external,
            must_use: signature.must_use.clone(),
            effects: signature.effects.clone(),
            capabilities: signature.capabilities.clone(),
            deterministic: signature.deterministic,
            realtime: signature.realtime,
            bounded: signature.bounded.clone(),
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
        reference_role: type_name.reference_role.clone(),
        span: type_name.span,
        resolved: type_name.resolved.clone(),
    }
}

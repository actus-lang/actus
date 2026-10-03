use std::collections::HashMap;

use crate::ast::{ExternalVerbDecl, Program, TopLevelDecl, TypeName, VerbDecl};
use crate::semantic::{GenericInstance, TypeSubstitution};

use super::super::native::NativeEmitError;
use super::definitions::canonical_type_name;
use super::verb_body::{specialize_block, specialize_param, specialize_return_type};

/// Materializes generic verbs for concrete type applications discovered by the
/// semantic pass. The resulting declarations have no generic parameters and
/// can therefore enter the ordinary Cranelift declaration pipeline.
pub(in crate::codegen) fn specialize_program(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Program, NativeEmitError> {
    let instances = expand_transitive_instances(program, instances)?;
    let call_bindings = unique_generic_call_bindings(program, &instances);
    let mut declarations = Vec::with_capacity(program.declarations.len());
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) if !verb.generic_parameters.is_empty() => {
                for specialized in specialize_verbs(verb, &instances, &call_bindings)? {
                    declarations.push(TopLevelDecl::Verb(specialized));
                }
            }
            TopLevelDecl::ExternalVerb(verb) if !verb.generic_parameters.is_empty() => {
                for specialized in specialize_external_verbs(verb, &instances, &call_bindings)? {
                    declarations.push(TopLevelDecl::ExternalVerb(specialized));
                }
            }
            TopLevelDecl::Verb(verb) => {
                let substitution = TypeSubstitution::for_type(
                    &verb.name,
                    &[],
                    &[],
                    crate::lexer::SourceSpan::new(0, 0),
                )
                .map_err(|error| {
                    NativeEmitError(format!("call specialization failed: {error:?}"))
                })?;
                let substitution = substitution.with_call_bindings(&call_bindings);
                declarations.push(TopLevelDecl::Verb(VerbDecl {
                    body: specialize_block(&verb.body, &substitution),
                    ..verb.clone()
                }));
            }
            other => declarations.push(other.clone()),
        }
    }
    Ok(Program { file_metadata: program.file_metadata.clone(), declarations })
}

fn unique_generic_call_bindings(
    program: &Program,
    instances: &[GenericInstance],
) -> HashMap<String, String> {
    let generic_names = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) if !verb.generic_parameters.is_empty() => {
                Some(verb.name.as_str())
            }
            TopLevelDecl::ExternalVerb(verb) if !verb.generic_parameters.is_empty() => {
                Some(verb.name.as_str())
            }
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    let mut candidates = HashMap::<String, Vec<String>>::new();
    for instance in instances {
        let is_concrete =
            generic_parameters_for(program, &instance.name).is_some_and(|parameters| {
                instance
                    .arguments
                    .iter()
                    .all(|argument| !contains_generic_parameter(argument, parameters))
            });
        if generic_names.contains(instance.name.as_str()) && is_concrete {
            candidates.entry(instance.name.clone()).or_default().push(specialized_name(instance));
        }
    }
    candidates
        .into_iter()
        .filter_map(|(name, mut specialized)| {
            specialized.sort();
            specialized.dedup();
            (specialized.len() == 1).then(|| (name, specialized.remove(0)))
        })
        .collect()
}

fn expand_transitive_instances(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Vec<GenericInstance>, NativeEmitError> {
    let mut expanded = instances.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for caller_instance in expanded.clone() {
            let Some(caller_parameters) = generic_parameters_for(program, &caller_instance.name)
            else {
                continue;
            };
            let substitution = TypeSubstitution::for_type(
                &caller_instance.name,
                caller_parameters,
                &caller_instance.arguments,
                crate::lexer::SourceSpan::new(0, 0),
            )
            .map_err(|error| {
                NativeEmitError(format!("generic instance expansion failed: {error:?}"))
            })?;
            for nested_template in expanded.clone().into_iter().filter(|instance| {
                instance.caller.as_deref() == Some(caller_instance.name.as_str())
            }) {
                let arguments = nested_template
                    .arguments
                    .iter()
                    .map(|argument| substitution.apply(argument))
                    .collect::<Vec<_>>();
                if arguments
                    .iter()
                    .any(|argument| contains_generic_parameter(argument, caller_parameters))
                {
                    continue;
                }
                let canonical_key = format!(
                    "{}[{}]",
                    nested_template.name,
                    arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
                );
                if expanded.iter().any(|instance| instance.canonical_key == canonical_key) {
                    continue;
                }
                expanded.push(GenericInstance {
                    name: nested_template.name,
                    arguments,
                    canonical_key,
                    caller: Some(caller_instance.name.clone()),
                });
                changed = true;
            }
        }
    }
    Ok(expanded)
}

fn generic_parameters_for<'a>(
    program: &'a Program,
    name: &str,
) -> Option<&'a [crate::ast::GenericParam]> {
    program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Verb(verb) if verb.name == name => Some(verb.generic_parameters.as_slice()),
        TopLevelDecl::ExternalVerb(verb) if verb.name == name => {
            Some(verb.generic_parameters.as_slice())
        }
        _ => None,
    })
}

fn contains_generic_parameter(
    type_name: &TypeName,
    parameters: &[crate::ast::GenericParam],
) -> bool {
    parameters.iter().any(|parameter| parameter.name == type_name.name)
        || type_name
            .arguments
            .iter()
            .any(|argument| contains_generic_parameter(argument, parameters))
}

fn specialize_external_verbs(
    verb: &ExternalVerbDecl,
    instances: &[GenericInstance],
    call_bindings: &HashMap<String, String>,
) -> Result<Vec<ExternalVerbDecl>, NativeEmitError> {
    instances
        .iter()
        .filter(|instance| instance_matches_external(instance, verb))
        .map(|instance| specialize_external_verb(verb, instance, call_bindings))
        .collect()
}

fn specialize_external_verb(
    verb: &ExternalVerbDecl,
    instance: &GenericInstance,
    call_bindings: &HashMap<String, String>,
) -> Result<ExternalVerbDecl, NativeEmitError> {
    let substitution = TypeSubstitution::for_type(
        &verb.name,
        &verb.generic_parameters,
        &instance.arguments,
        verb.span,
    )
    .map_err(|error| NativeEmitError(format!("generic verb substitution failed: {error:?}")))?
    .with_call_bindings(call_bindings);
    Ok(ExternalVerbDecl {
        is_open: verb.is_open,
        doc: verb.doc.clone(),
        unsafe_boundary: verb.unsafe_boundary,
        module_import: verb.module_import,
        abi: verb.abi,
        metadata: verb.metadata.clone(),
        name: specialized_name(instance),
        generic_parameters: Vec::new(),
        params: verb.params.iter().map(|param| specialize_param(param, &substitution)).collect(),
        return_type: verb
            .return_type
            .as_ref()
            .map(|return_type| specialize_return_type(return_type, &substitution)),
        span: verb.span,
    })
}

fn specialize_verbs(
    verb: &VerbDecl,
    instances: &[GenericInstance],
    call_bindings: &HashMap<String, String>,
) -> Result<Vec<VerbDecl>, NativeEmitError> {
    instances
        .iter()
        .filter(|instance| instance_matches_verb(instance, verb))
        .map(|instance| specialize_verb(verb, instance, call_bindings))
        .collect()
}

fn specialize_verb(
    verb: &VerbDecl,
    instance: &GenericInstance,
    call_bindings: &HashMap<String, String>,
) -> Result<VerbDecl, NativeEmitError> {
    let substitution = TypeSubstitution::for_type(
        &verb.name,
        &verb.generic_parameters,
        &instance.arguments,
        verb.span,
    )
    .map_err(|error| NativeEmitError(format!("generic verb substitution failed: {error:?}")))?
    .with_call_bindings(call_bindings);
    Ok(VerbDecl {
        is_open: verb.is_open,
        doc: verb.doc.clone(),
        metadata: verb.metadata.clone(),
        name: specialized_name(instance),
        generic_parameters: Vec::new(),
        params: verb.params.iter().map(|param| specialize_param(param, &substitution)).collect(),
        return_type: verb
            .return_type
            .as_ref()
            .map(|return_type| specialize_return_type(return_type, &substitution)),
        body: specialize_block(&verb.body, &substitution),
        span: verb.span,
    })
}

pub(crate) fn specialized_generic_name(instance: &GenericInstance) -> String {
    format!("{}__{}", instance.name, encode_specialization_arguments(&instance.arguments))
}

fn specialized_name(instance: &GenericInstance) -> String {
    specialized_generic_name(instance)
}

fn encode_specialization_arguments(arguments: &[TypeName]) -> String {
    arguments
        .iter()
        .map(canonical_type_name)
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() { character } else { '_' })
        .collect()
}

fn instance_matches_verb(instance: &GenericInstance, verb: &VerbDecl) -> bool {
    instance.name == verb.name
        && instance.arguments.len() == verb.generic_parameters.len()
        && instance
            .arguments
            .iter()
            .all(|argument| !contains_generic_parameter(argument, &verb.generic_parameters))
}

fn instance_matches_external(instance: &GenericInstance, verb: &ExternalVerbDecl) -> bool {
    instance.name == verb.name
        && instance.arguments.len() == verb.generic_parameters.len()
        && instance
            .arguments
            .iter()
            .all(|argument| !contains_generic_parameter(argument, &verb.generic_parameters))
}

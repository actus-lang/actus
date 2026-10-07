use std::collections::HashMap;

use crate::ast::{ExternalVerbDecl, Program, TopLevelDecl, TypeIdentity, TypeName, VerbDecl};
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
    let call_site_bindings = generic_call_site_bindings(&instances);
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
                let substitution = substitution
                    .with_call_bindings(&call_bindings)
                    .with_call_site_bindings(&call_site_bindings);
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

pub(crate) fn expand_generic_instances(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Vec<GenericInstance>, NativeEmitError> {
    expand_transitive_instances(program, instances)
}

fn generic_call_site_bindings(
    instances: &[GenericInstance],
) -> HashMap<(String, usize, usize), String> {
    instances
        .iter()
        .filter(|instance| instance.caller.is_none())
        .map(|instance| {
            (
                (instance.name.clone(), instance.call_span.start, instance.call_span.end),
                specialized_name(instance),
            )
        })
        .collect()
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
    while expand_transitive_once(program, &mut expanded)? {
        // Keep expanding until the dependency graph reaches a fixed point.
    }
    Ok(expanded)
}

fn expand_transitive_once(
    program: &Program,
    expanded: &mut Vec<GenericInstance>,
) -> Result<bool, NativeEmitError> {
    let snapshot = expanded.clone();
    let mut changed = false;
    for caller_instance in snapshot.iter() {
        let Some(caller_parameters) = generic_parameters_for(program, &caller_instance.name) else {
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
        for nested_template in snapshot
            .iter()
            .filter(|instance| instance.caller.as_deref() == Some(caller_instance.name.as_str()))
        {
            changed |= append_specialized_nested_instance(
                expanded,
                caller_instance,
                caller_parameters,
                &substitution,
                nested_template,
            );
        }
    }
    Ok(changed)
}

fn append_specialized_nested_instance(
    expanded: &mut Vec<GenericInstance>,
    caller_instance: &GenericInstance,
    caller_parameters: &[crate::ast::GenericParam],
    substitution: &TypeSubstitution,
    nested_template: &GenericInstance,
) -> bool {
    let arguments = nested_template
        .arguments
        .iter()
        .map(|argument| substitution.apply(argument))
        .collect::<Vec<_>>();
    if arguments.iter().any(|argument| contains_generic_parameter(argument, caller_parameters)) {
        return false;
    }
    let identity = TypeIdentity::from_application(&nested_template.name, &arguments);
    let canonical_key = identity.key();
    if expanded.iter().any(|instance| {
        instance.identity() == identity && instance.caller == Some(caller_instance.name.clone())
    }) {
        return false;
    }
    expanded.push(GenericInstance {
        name: nested_template.name.clone(),
        arguments,
        canonical_key,
        caller: Some(caller_instance.name.clone()),
        call_span: nested_template.call_span,
    });
    true
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
    let mut specialized = Vec::new();
    let mut names = std::collections::HashSet::new();
    for instance in instances.iter().filter(|instance| instance_matches_external(instance, verb)) {
        if names.insert(specialized_name(instance)) {
            specialized.push(specialize_external_verb(verb, instance, instances, call_bindings)?);
        }
    }
    Ok(specialized)
}

fn specialize_external_verb(
    verb: &ExternalVerbDecl,
    instance: &GenericInstance,
    instances: &[GenericInstance],
    call_bindings: &HashMap<String, String>,
) -> Result<ExternalVerbDecl, NativeEmitError> {
    let substitution = TypeSubstitution::for_type(
        &verb.name,
        &verb.generic_parameters,
        &instance.arguments,
        verb.span,
    )
    .map_err(|error| NativeEmitError(format!("generic verb substitution failed: {error:?}")))?;
    let call_bindings = nested_call_bindings(
        &verb.name,
        &verb.generic_parameters,
        instance,
        instances,
        call_bindings,
    )?;
    let substitution = substitution.with_call_bindings(&call_bindings);
    Ok(ExternalVerbDecl {
        is_open: verb.is_open,
        doc: verb.doc.clone(),
        contract: verb.contract.clone(),
        unsafe_boundary: verb.unsafe_boundary,
        module_import: verb.module_import,
        abi: verb.abi,
        metadata: verb.metadata.clone(),
        name: specialized_name(instance),
        native_symbol: (!verb.module_import)
            .then(|| verb.native_symbol.clone().unwrap_or_else(|| verb.name.clone())),
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
    let mut specialized = Vec::new();
    let mut names = std::collections::HashSet::new();
    for instance in instances.iter().filter(|instance| instance_matches_verb(instance, verb)) {
        if names.insert(specialized_name(instance)) {
            specialized.push(specialize_verb(verb, instance, instances, call_bindings)?);
        }
    }
    Ok(specialized)
}

fn specialize_verb(
    verb: &VerbDecl,
    instance: &GenericInstance,
    instances: &[GenericInstance],
    call_bindings: &HashMap<String, String>,
) -> Result<VerbDecl, NativeEmitError> {
    let substitution = TypeSubstitution::for_type(
        &verb.name,
        &verb.generic_parameters,
        &instance.arguments,
        verb.span,
    )
    .map_err(|error| NativeEmitError(format!("generic verb substitution failed: {error:?}")))?;
    let call_bindings = nested_call_bindings(
        &verb.name,
        &verb.generic_parameters,
        instance,
        instances,
        call_bindings,
    )?;
    let substitution = substitution.with_call_bindings(&call_bindings);
    Ok(VerbDecl {
        is_open: verb.is_open,
        doc: verb.doc.clone(),
        contract: verb.contract.clone(),
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

fn nested_call_bindings(
    caller_name: &str,
    parameters: &[crate::ast::GenericParam],
    caller_instance: &GenericInstance,
    instances: &[GenericInstance],
    inherited: &HashMap<String, String>,
) -> Result<HashMap<String, String>, NativeEmitError> {
    let substitution = TypeSubstitution::for_type(
        caller_name,
        parameters,
        &caller_instance.arguments,
        crate::lexer::SourceSpan::new(0, 0),
    )
    .map_err(|error| NativeEmitError(format!("nested generic substitution failed: {error:?}")))?;
    let mut bindings = inherited.clone();
    for template in instances.iter().filter(|instance| {
        instance.caller.as_deref() == Some(caller_name)
            && instance.arguments.iter().any(|argument| substitution.apply(argument) != *argument)
    }) {
        let arguments = template
            .arguments
            .iter()
            .map(|argument| substitution.apply(argument))
            .collect::<Vec<_>>();
        let identity = TypeIdentity::from_application(&template.name, &arguments);
        if let Some(concrete) = instances.iter().find(|instance| instance.identity() == identity) {
            bindings.insert(template.name.clone(), specialized_name(concrete));
        }
    }
    Ok(bindings)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::generic::definitions::specialized_enums;

    #[test]
    fn generic_specialization_preserves_structured_contracts() {
        let source = r#"
            """
            contract:
            purpose:
                Preserve this description after specialization.
            ownership:
                The caller keeps the input owner.
            """
            verb identity[T](abs item: T) -> T { return item; }
        "#;
        let (tokens, errors) = crate::lexer::scan(source);
        assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
        let program = crate::parser::parse(tokens).expect("generic contract should parse");
        let instance = GenericInstance {
            name: "identity".to_owned(),
            arguments: vec![TypeName {
                name: "Int".to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span: crate::lexer::SourceSpan::new(0, 0),
            }],
            canonical_key: "identity[Int]".to_owned(),
            caller: None,
            call_span: crate::lexer::SourceSpan::new(0, 0),
        };
        let specialized = specialize_program(&program, &[instance]).expect("specialization");
        let TopLevelDecl::Verb(verb) = &specialized.declarations[0] else {
            panic!("expected specialized verb")
        };
        assert_eq!(verb.name, "identity__Int");
        assert_eq!(
            verb.contract
                .as_ref()
                .and_then(|contract| contract.sections.first())
                .map(|section| section.text.as_str()),
            Some("Preserve this description after specialization.")
        );
    }

    #[test]
    fn specialization_trace_separates_structural_and_abi_identity() {
        let source =
            "enum Result[T] { Ok(T), Err(Int), } verb main(erg value: Result[abs Item]) { }";
        let (tokens, errors) = crate::lexer::scan(source);
        assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
        let program = crate::parser::parse(tokens).expect("generic result should parse");
        let instance = GenericInstance {
            name: "Result".to_owned(),
            arguments: vec![TypeName {
                name: "Item".to_owned(),
                arguments: Vec::new(),
                reference_role: Some(crate::ast::Role::Abs),
                span: crate::lexer::SourceSpan::new(0, 4),
            }],
            canonical_key: "Result[abs Item]".to_owned(),
            caller: None,
            call_span: crate::lexer::SourceSpan::new(0, 4),
        };

        assert_eq!(instance.identity().key(), "Result[Item]");
        assert_eq!(instance.canonical_key, "Result[abs Item]");

        let specialized = specialized_enums(&program, &[instance]).expect("specialization");
        let definition = specialized.first().expect("specialized enum");
        assert_eq!(definition.name, "Result[abs Item]");
    }
}

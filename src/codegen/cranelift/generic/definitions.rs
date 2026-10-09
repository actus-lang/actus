use std::collections::{HashMap, HashSet};

use crate::ast::{
    EnumDef, EnumPayload, EnumVariant, GenericParam, Program, StructDef, StructField, TopLevelDecl,
    TypeIdentity, TypeName, builtin_enum_definitions,
};
use crate::semantic::{GenericInstance, TypeSubstitution};

use super::super::native::NativeEmitError;

pub(in crate::codegen) fn specialized_structs(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Vec<StructDef>, NativeEmitError> {
    let definitions = struct_definitions(program);
    let generic_names = generic_struct_names(&definitions)
        .into_iter()
        .chain(generic_enum_names(&enum_definitions(program)))
        .collect::<HashSet<_>>();
    instances
        .iter()
        .filter_map(|instance| {
            definitions.get(&instance.name).map(|definition| (instance, definition))
        })
        .filter(|(_, definition)| !definition.generic_parameters.is_empty())
        .map(|(instance, definition)| {
            let substitution = substitution_for_instance(instance, definition)?;
            let fields = definition
                .fields
                .iter()
                .map(|field| specialize_field(field, &substitution, &generic_names))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(StructDef {
                is_open: false,
                doc: definition.doc.clone(),
                name: instance.canonical_key.clone(),
                generic_parameters: Vec::new(),
                fields,
                span: definition.span,
            })
        })
        .collect()
}

pub(in crate::codegen) fn specialized_enums(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Vec<EnumDef>, NativeEmitError> {
    let definitions = enum_definitions(program);
    let generic_names = generic_enum_names(&definitions);
    instances
        .iter()
        .filter_map(|instance| {
            definitions.get(&instance.name).map(|definition| (instance, definition))
        })
        .filter(|(_, definition)| !definition.generic_parameters.is_empty())
        .map(|(instance, definition)| {
            let substitution = substitution_for_instance(instance, definition)?;
            let variants = definition
                .variants
                .iter()
                .map(|variant| specialize_variant(variant, &substitution, &generic_names))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnumDef {
                is_open: false,
                doc: definition.doc.clone(),
                name: instance.canonical_key.clone(),
                generic_parameters: Vec::new(),
                variants,
                span: definition.span,
            })
        })
        .collect()
}

pub(crate) fn collect_concrete_type_instances(program: &Program) -> Vec<GenericInstance> {
    let (generic_names, generic_parameter_names) = generic_type_metadata(program);
    let mut instances = Vec::new();
    for declaration in &program.declarations {
        collect_declaration_type_instances(
            declaration,
            &generic_names,
            &generic_parameter_names,
            &mut instances,
        );
    }
    instances
}

fn generic_type_metadata(program: &Program) -> (HashSet<String>, HashSet<String>) {
    let mut generic_names = HashSet::new();
    let mut parameter_names = HashSet::new();
    for definition in builtin_enum_definitions()
        .into_iter()
        .filter(|definition| !definition.generic_parameters.is_empty())
    {
        generic_names.insert(definition.name);
        parameter_names
            .extend(definition.generic_parameters.into_iter().map(|parameter| parameter.name));
    }
    for declaration in &program.declarations {
        let (name, parameters) = match declaration {
            TopLevelDecl::Struct(definition) => (&definition.name, &definition.generic_parameters),
            TopLevelDecl::Enum(definition) => (&definition.name, &definition.generic_parameters),
            _ => continue,
        };
        if parameters.is_empty() {
            continue;
        }
        generic_names.insert(name.clone());
        parameter_names.extend(parameters.iter().map(|parameter| parameter.name.clone()));
    }
    (generic_names, parameter_names)
}

fn collect_declaration_type_instances(
    declaration: &TopLevelDecl,
    generic_names: &HashSet<String>,
    parameter_names: &HashSet<String>,
    instances: &mut Vec<GenericInstance>,
) {
    match declaration {
        TopLevelDecl::Verb(verb) => {
            verb.params.iter().for_each(|parameter| {
                collect_type_instances(&parameter.ty, generic_names, parameter_names, instances)
            });
            verb.return_type.as_ref().iter().for_each(|return_type| {
                collect_type_instances(&return_type.ty, generic_names, parameter_names, instances)
            });
        }
        TopLevelDecl::ExternalVerb(verb) => {
            verb.params.iter().for_each(|parameter| {
                collect_type_instances(&parameter.ty, generic_names, parameter_names, instances)
            });
            verb.return_type.as_ref().iter().for_each(|return_type| {
                collect_type_instances(&return_type.ty, generic_names, parameter_names, instances)
            });
        }
        TopLevelDecl::Struct(definition) => definition.fields.iter().for_each(|field| {
            collect_type_instances(&field.ty, generic_names, parameter_names, instances)
        }),
        TopLevelDecl::Enum(definition) => {
            collect_enum_type_instances(definition, generic_names, parameter_names, instances)
        }
        _ => {}
    }
}

fn collect_enum_type_instances(
    definition: &EnumDef,
    generic_names: &HashSet<String>,
    parameter_names: &HashSet<String>,
    instances: &mut Vec<GenericInstance>,
) {
    for variant in &definition.variants {
        match &variant.payload {
            EnumPayload::Tuple(fields) => fields.iter().for_each(|field| {
                collect_type_instances(field, generic_names, parameter_names, instances)
            }),
            EnumPayload::Struct(fields) => fields.iter().for_each(|field| {
                collect_type_instances(&field.ty, generic_names, parameter_names, instances)
            }),
            EnumPayload::Unit => {}
        }
    }
}

fn collect_type_instances(
    type_name: &TypeName,
    generic_names: &HashSet<String>,
    generic_parameter_names: &HashSet<String>,
    instances: &mut Vec<GenericInstance>,
) {
    type_name.arguments.iter().for_each(|argument| {
        collect_type_instances(argument, generic_names, generic_parameter_names, instances)
    });
    if !generic_names.contains(&type_name.name)
        || type_name.arguments.is_empty()
        || type_name
            .arguments
            .iter()
            .any(|argument| contains_generic_parameter(argument, generic_parameter_names))
    {
        return;
    }
    let identity = TypeIdentity::from_type_name(type_name);
    if instances.iter().any(|instance| instance.identity() == identity) {
        return;
    }
    instances.push(GenericInstance {
        name: type_name.name.clone(),
        arguments: type_name.arguments.clone(),
        canonical_key: canonical_type_name(type_name),
        caller: None,
        call_span: type_name.span,
    });
}

fn contains_generic_parameter(
    type_name: &TypeName,
    generic_parameter_names: &HashSet<String>,
) -> bool {
    generic_parameter_names.contains(&type_name.name)
        || type_name
            .arguments
            .iter()
            .any(|argument| contains_generic_parameter(argument, generic_parameter_names))
}

fn struct_definitions(program: &Program) -> HashMap<String, StructDef> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Struct(definition) => Some((definition.name.clone(), definition.clone())),
            _ => None,
        })
        .collect()
}

fn enum_definitions(program: &Program) -> HashMap<String, EnumDef> {
    let mut definitions = builtin_enum_definitions()
        .into_iter()
        .map(|definition| (definition.name.clone(), definition))
        .collect::<HashMap<_, _>>();
    definitions.extend(
        program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Enum(definition) => {
                    Some((definition.name.clone(), definition.clone()))
                }
                _ => None,
            })
            .collect::<HashMap<_, _>>(),
    );
    definitions
}

fn generic_struct_names(definitions: &HashMap<String, StructDef>) -> HashSet<String> {
    definitions
        .iter()
        .filter(|(_, definition)| !definition.generic_parameters.is_empty())
        .map(|(name, _)| name.clone())
        .collect()
}

fn generic_enum_names(definitions: &HashMap<String, EnumDef>) -> HashSet<String> {
    definitions
        .iter()
        .filter(|(_, definition)| !definition.generic_parameters.is_empty())
        .map(|(name, _)| name.clone())
        .collect()
}

fn substitution_for_instance<T>(
    instance: &GenericInstance,
    definition: &T,
) -> Result<TypeSubstitution, NativeEmitError>
where
    T: GenericDefinition,
{
    TypeSubstitution::for_type(
        definition.name(),
        definition.parameters(),
        &instance.arguments,
        definition.span(),
    )
    .map_err(|error| NativeEmitError(format!("generic substitution failed: {error:?}")))
}

trait GenericDefinition {
    fn name(&self) -> &str;
    fn parameters(&self) -> &[GenericParam];
    fn span(&self) -> crate::lexer::SourceSpan;
}

impl GenericDefinition for StructDef {
    fn name(&self) -> &str {
        &self.name
    }
    fn parameters(&self) -> &[GenericParam] {
        &self.generic_parameters
    }
    fn span(&self) -> crate::lexer::SourceSpan {
        self.span
    }
}

impl GenericDefinition for EnumDef {
    fn name(&self) -> &str {
        &self.name
    }
    fn parameters(&self) -> &[GenericParam] {
        &self.generic_parameters
    }
    fn span(&self) -> crate::lexer::SourceSpan {
        self.span
    }
}

fn specialize_field(
    field: &StructField,
    substitution: &TypeSubstitution,
    generic_names: &HashSet<String>,
) -> Result<StructField, NativeEmitError> {
    Ok(StructField {
        doc: field.doc.clone(),
        role: field.role.clone(),
        name: field.name.clone(),
        ty: specialize_type(&substitution.apply(&field.ty), generic_names),
        span: field.span,
    })
}

fn specialize_variant(
    variant: &EnumVariant,
    substitution: &TypeSubstitution,
    generic_names: &HashSet<String>,
) -> Result<EnumVariant, NativeEmitError> {
    let payload = match &variant.payload {
        EnumPayload::Unit => EnumPayload::Unit,
        EnumPayload::Tuple(types) => EnumPayload::Tuple(
            types
                .iter()
                .map(|ty| specialize_type(&substitution.apply(ty), generic_names))
                .collect(),
        ),
        EnumPayload::Struct(fields) => EnumPayload::Struct(
            fields
                .iter()
                .map(|field| crate::ast::EnumField {
                    doc: field.doc.clone(),
                    name: field.name.clone(),
                    ty: specialize_type(&substitution.apply(&field.ty), generic_names),
                    span: field.span,
                })
                .collect(),
        ),
    };
    Ok(EnumVariant {
        doc: variant.doc.clone(),
        name: variant.name.clone(),
        payload,
        span: variant.span,
    })
}

fn specialize_type(type_name: &TypeName, generic_names: &HashSet<String>) -> TypeName {
    if type_name.arguments.is_empty() || !generic_names.contains(&type_name.name) {
        return type_name.clone();
    }
    TypeName {
        name: canonical_type_name(type_name),
        arguments: Vec::new(),
        reference_role: type_name.reference_role.clone(),
        span: type_name.span,
    }
}

pub(in crate::codegen) fn canonical_type_name(type_name: &TypeName) -> String {
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

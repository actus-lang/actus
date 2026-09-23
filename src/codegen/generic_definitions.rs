use std::collections::{HashMap, HashSet};

use crate::ast::{
    EnumDef, EnumPayload, EnumVariant, GenericParam, Program, StructDef, StructField, TopLevelDecl,
    TypeName, builtin_enum_definitions,
};
use crate::semantic::{GenericInstance, TypeSubstitution};

use super::native::NativeEmitError;

pub(super) fn specialized_structs(
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
                name: instance.canonical_key.clone(),
                generic_parameters: Vec::new(),
                fields,
                span: definition.span,
            })
        })
        .collect()
}

pub(super) fn specialized_enums(
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
                name: instance.canonical_key.clone(),
                generic_parameters: Vec::new(),
                variants,
                span: definition.span,
            })
        })
        .collect()
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
                    name: field.name.clone(),
                    ty: specialize_type(&substitution.apply(&field.ty), generic_names),
                    span: field.span,
                })
                .collect(),
        ),
    };
    Ok(EnumVariant { name: variant.name.clone(), payload, span: variant.span })
}

fn specialize_type(type_name: &TypeName, generic_names: &HashSet<String>) -> TypeName {
    if type_name.arguments.is_empty() || !generic_names.contains(&type_name.name) {
        return type_name.clone();
    }
    TypeName { name: canonical_type_name(type_name), arguments: Vec::new(), span: type_name.span }
}

pub(super) fn canonical_type_name(type_name: &TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

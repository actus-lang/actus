use crate::ast::{
    BuiltinType, EnumPayload, PrimitiveType, Program, Repr, StructFieldRole, TopLevelDecl,
    TypeName, lookup_builtin_type, primitive_type,
};

use super::{CAbiAggregate, CAbiAggregateField, CAbiError, CAbiLayout, CAbiTarget, CAbiType};

pub(super) fn map_type_in_program(
    target: CAbiTarget,
    program: &Program,
    type_name: &TypeName,
) -> Result<CAbiType, CAbiError> {
    map_type_in_program_with_visiting(target, program, type_name, &mut Vec::new())
}

fn map_type_in_program_with_visiting(
    target: CAbiTarget,
    program: &Program,
    type_name: &TypeName,
    visiting: &mut Vec<String>,
) -> Result<CAbiType, CAbiError> {
    if let Some(mapped) = map_scalar_type(type_name) {
        return mapped;
    }
    if type_name.name == "Array" {
        return map_array(target, program, type_name, visiting);
    }
    let definition = program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Struct(definition) if definition.name == type_name.name => Some(definition),
        _ => None,
    });
    if let Some(definition) = definition {
        if !type_name.arguments.is_empty() || !definition.generic_parameters.is_empty() {
            return Err(CAbiError::GenericAggregate { name: type_name.name.clone() });
        }
        if definition.repr() != Some(Repr::C) {
            return Err(CAbiError::AggregateRequiresReprC { name: type_name.name.clone() });
        }
        return map_struct(target, program, definition, visiting);
    }
    if let Some(definition) =
        program.declarations.iter().find_map(|declaration| match declaration {
            TopLevelDecl::Enum(definition) if definition.name == type_name.name => Some(definition),
            _ => None,
        })
    {
        if !type_name.arguments.is_empty() || !definition.generic_parameters.is_empty() {
            return Err(CAbiError::GenericAggregate { name: type_name.name.clone() });
        }
        if definition.repr() != Some(Repr::C) {
            return Err(CAbiError::AggregateRequiresReprC { name: type_name.name.clone() });
        }
        if definition.variants.iter().any(|variant| !matches!(variant.payload, EnumPayload::Unit)) {
            return Err(CAbiError::AggregateFieldType {
                aggregate: definition.name.clone(),
                field: "<enum payload>".to_owned(),
                ty: definition.name.clone(),
            });
        }
        return Ok(CAbiType::Enum {
            name: definition.name.clone(),
            layout: CAbiLayout { size: 4, alignment: 4 },
        });
    }
    Err(CAbiError::UnsupportedType { name: type_name.name.clone() })
}

fn map_scalar_type(type_name: &TypeName) -> Option<Result<CAbiType, CAbiError>> {
    if !type_name.arguments.is_empty() {
        return None;
    }
    if type_name.name == "Usize" {
        return Some(Ok(CAbiType::Usize));
    }
    match primitive_type(&type_name.name) {
        Some(PrimitiveType::Integer { signed, width }) if width.is_multiple_of(8) => {
            Some(if width == 32 {
                Ok(CAbiType::Int32)
            } else {
                Ok(CAbiType::Integer { signed, width })
            })
        }
        Some(PrimitiveType::Float { width }) if width.is_multiple_of(8) => {
            Some(Ok(CAbiType::Float { width }))
        }
        Some(PrimitiveType::Void) => None,
        _ => match lookup_builtin_type(&type_name.name) {
            Some(BuiltinType::Int) => Some(Ok(CAbiType::Int32)),
            Some(BuiltinType::Bool) => Some(Ok(CAbiType::Bool)),
            Some(BuiltinType::Buffer | BuiltinType::String) => Some(Ok(CAbiType::OpaquePointer)),
            _ => None,
        },
    }
}

fn map_array(
    target: CAbiTarget,
    program: &Program,
    type_name: &TypeName,
    visiting: &mut Vec<String>,
) -> Result<CAbiType, CAbiError> {
    let [element, length] = type_name.arguments.as_slice() else {
        return Err(CAbiError::UnsupportedType { name: type_name.canonical_key() });
    };
    let length = length
        .name
        .parse::<u32>()
        .ok()
        .filter(|length| *length > 0)
        .ok_or_else(|| CAbiError::UnsupportedType { name: type_name.canonical_key() })?;
    let element = map_type_in_program_with_visiting(target, program, element, visiting)?;
    let element_layout = target.layout(element.clone());
    let size = element_layout
        .size
        .checked_mul(length)
        .ok_or_else(|| CAbiError::AggregateLayoutOverflow { name: type_name.canonical_key() })?;
    Ok(CAbiType::Array {
        element: Box::new(element),
        length,
        layout: CAbiLayout { size, alignment: element_layout.alignment },
    })
}

fn map_struct(
    target: CAbiTarget,
    program: &Program,
    definition: &crate::ast::StructDef,
    visiting: &mut Vec<String>,
) -> Result<CAbiType, CAbiError> {
    if visiting.iter().any(|name| name == &definition.name) {
        return Err(CAbiError::AggregateLayoutOverflow { name: definition.name.clone() });
    }
    visiting.push(definition.name.clone());
    let mut offset = 0u32;
    let mut alignment = 1u32;
    let mut fields = Vec::new();
    for field in &definition.fields {
        if !matches!(field.role, StructFieldRole::Value) {
            return Err(CAbiError::AggregateFieldRole {
                aggregate: definition.name.clone(),
                field: field.name.clone(),
                role: format!("{:?}", field.role).to_lowercase(),
            });
        }
        let field_ty = map_type_in_program_with_visiting(target, program, &field.ty, visiting)?;
        let field_layout = target.layout(field_ty.clone());
        if field_layout.size == 0 || field_layout.alignment == 0 {
            return Err(CAbiError::AggregateFieldType {
                aggregate: definition.name.clone(),
                field: field.name.clone(),
                ty: field.ty.canonical_key(),
            });
        }
        offset = align_up(offset, field_layout.alignment)
            .and_then(|value| value.checked_add(field_layout.size))
            .ok_or_else(|| CAbiError::AggregateLayoutOverflow { name: definition.name.clone() })?;
        alignment = alignment.max(field_layout.alignment);
        fields.push(CAbiAggregateField {
            name: field.name.clone(),
            offset: offset - field_layout.size,
            layout: field_layout,
            ty: field_ty,
        });
    }
    visiting.pop();
    Ok(CAbiType::Aggregate(CAbiAggregate {
        name: definition.name.clone(),
        layout: CAbiLayout { size: align_up(offset, alignment).unwrap_or(0), alignment },
        fields,
    }))
}

fn align_up(value: u32, alignment: u32) -> Option<u32> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return None;
    }
    value.checked_add(alignment - 1).map(|value| value & !(alignment - 1))
}

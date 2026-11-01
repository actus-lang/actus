use crate::ast::{
    BuiltinType, EnumPayload, PrimitiveType, Program, StructFieldRole, TopLevelDecl, TypeName,
    lookup_builtin_type, primitive_type,
};

use super::{
    CAbiAggregate, CAbiAggregateField, CAbiError, CAbiLayout, CAbiTaggedUnion,
    CAbiTaggedUnionVariant, CAbiTarget, CAbiType,
};

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
        let repr = definition.repr();
        if repr.is_some_and(|repr| repr.is_transparent()) {
            return map_transparent_struct(target, program, definition, visiting);
        }
        if !repr.is_some_and(|repr| repr.is_c()) {
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
        if !definition.repr().is_some_and(|repr| repr.is_c()) {
            return Err(CAbiError::AggregateRequiresReprC { name: type_name.name.clone() });
        }
        return map_enum(target, program, definition, visiting);
    }
    Err(CAbiError::UnsupportedType { name: type_name.name.clone() })
}

fn map_enum(
    target: CAbiTarget,
    program: &Program,
    definition: &crate::ast::EnumDef,
    visiting: &mut Vec<String>,
) -> Result<CAbiType, CAbiError> {
    if definition.variants.iter().all(|variant| matches!(variant.payload, EnumPayload::Unit)) {
        return Ok(CAbiType::Enum {
            name: definition.name.clone(),
            layout: CAbiLayout { size: 4, alignment: 4 },
        });
    }
    if visiting.iter().any(|name| name == &definition.name) {
        return Err(CAbiError::AggregateLayoutOverflow { name: definition.name.clone() });
    }
    visiting.push(definition.name.clone());
    let mut variants = Vec::new();
    let mut payload_layout: Option<CAbiLayout> = None;
    for (discriminant, variant) in definition.variants.iter().enumerate() {
        let payload = map_enum_payload(target, program, definition, variant, visiting)?;
        if let Some(payload) = &payload {
            let layout = payload.layout;
            payload_layout = Some(match payload_layout {
                Some(current) if current.size >= layout.size => current,
                _ => layout,
            });
        }
        variants.push(CAbiTaggedUnionVariant {
            name: variant.name.clone(),
            discriminant: u32::try_from(discriminant).map_err(|_| {
                CAbiError::AggregateLayoutOverflow { name: definition.name.clone() }
            })?,
            payload,
        });
    }
    visiting.pop();
    let tag = CAbiLayout { size: 4, alignment: 4 };
    let payload_offset =
        payload_layout.map_or(4, |layout| align_up(4, layout.alignment).unwrap_or(0));
    let alignment =
        payload_layout.map_or(tag.alignment, |layout| tag.alignment.max(layout.alignment));
    let size = align_up(
        payload_offset
            .checked_add(payload_layout.map_or(0, |layout| layout.size))
            .ok_or_else(|| CAbiError::AggregateLayoutOverflow { name: definition.name.clone() })?,
        alignment,
    )
    .ok_or_else(|| CAbiError::AggregateLayoutOverflow { name: definition.name.clone() })?;
    Ok(CAbiType::TaggedUnion(CAbiTaggedUnion {
        name: definition.name.clone(),
        tag,
        payload_offset,
        payload: payload_layout,
        layout: CAbiLayout { size, alignment },
        variants,
    }))
}

fn map_enum_payload(
    target: CAbiTarget,
    program: &Program,
    definition: &crate::ast::EnumDef,
    variant: &crate::ast::EnumVariant,
    visiting: &mut Vec<String>,
) -> Result<Option<CAbiAggregate>, CAbiError> {
    let fields = match &variant.payload {
        EnumPayload::Unit => return Ok(None),
        EnumPayload::Tuple(types) => {
            types.iter().enumerate().map(|(index, ty)| (index.to_string(), ty)).collect::<Vec<_>>()
        }
        EnumPayload::Struct(fields) => {
            fields.iter().map(|field| (field.name.clone(), &field.ty)).collect::<Vec<_>>()
        }
    };
    let mut offset = 0u32;
    let mut alignment = 1u32;
    let mut mapped_fields = Vec::new();
    for (name, type_name) in fields {
        let ty = map_type_in_program_with_visiting(target, program, type_name, visiting)?;
        let field_layout = target.layout(ty.clone());
        offset = align_up(offset, field_layout.alignment)
            .and_then(|value| value.checked_add(field_layout.size))
            .ok_or_else(|| CAbiError::AggregateLayoutOverflow { name: definition.name.clone() })?;
        alignment = alignment.max(field_layout.alignment);
        mapped_fields.push(CAbiAggregateField {
            name,
            offset: offset - field_layout.size,
            layout: field_layout,
            ty,
        });
    }
    Ok(Some(CAbiAggregate {
        name: format!("{}__{}Payload", definition.name, variant.name),
        packed: false,
        layout: CAbiLayout {
            size: align_up(offset, alignment).ok_or_else(|| {
                CAbiError::AggregateLayoutOverflow { name: definition.name.clone() }
            })?,
            alignment,
        },
        fields: mapped_fields,
    }))
}

fn map_transparent_struct(
    target: CAbiTarget,
    program: &Program,
    definition: &crate::ast::StructDef,
    visiting: &mut Vec<String>,
) -> Result<CAbiType, CAbiError> {
    if visiting.iter().any(|name| name == &definition.name) {
        return Err(CAbiError::AggregateLayoutOverflow { name: definition.name.clone() });
    }
    if definition.repr().is_some_and(|repr| repr.is_c() || repr.is_packed()) {
        return Err(CAbiError::TransparentCombinedRepresentation { name: definition.name.clone() });
    }
    let [field] = definition.fields.as_slice() else {
        return Err(CAbiError::TransparentRequiresOneField { name: definition.name.clone() });
    };
    if !matches!(field.role, StructFieldRole::Value) {
        return Err(CAbiError::TransparentFieldRole {
            aggregate: definition.name.clone(),
            field: field.name.clone(),
            role: format!("{:?}", field.role).to_lowercase(),
        });
    }
    visiting.push(definition.name.clone());
    let mapped = map_type_in_program_with_visiting(target, program, &field.ty, visiting)?;
    visiting.pop();
    if matches!(mapped, CAbiType::OpaquePointer) {
        return Err(CAbiError::TransparentFieldType {
            aggregate: definition.name.clone(),
            field: field.name.clone(),
            ty: field.ty.canonical_key(),
        });
    }
    Ok(mapped)
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
    let packed = definition.repr().is_some_and(|repr| repr.is_packed());
    let explicit_alignment = definition.repr().and_then(|repr| repr.alignment()).unwrap_or(1);
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
        let field_alignment = if packed { 1 } else { field_layout.alignment };
        offset = align_up(offset, field_alignment)
            .and_then(|value| value.checked_add(field_layout.size))
            .ok_or_else(|| CAbiError::AggregateLayoutOverflow { name: definition.name.clone() })?;
        alignment = alignment.max(field_alignment);
        fields.push(CAbiAggregateField {
            name: field.name.clone(),
            offset: offset - field_layout.size,
            layout: field_layout,
            ty: field_ty,
        });
    }
    visiting.pop();
    alignment = alignment.max(explicit_alignment);
    Ok(CAbiType::Aggregate(CAbiAggregate {
        name: definition.name.clone(),
        packed,
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

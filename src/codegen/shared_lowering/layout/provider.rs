use super::super::BackendFieldLayout;
use super::BackendLayoutProvider;

impl BackendLayoutProvider for ProgramLayouts<'_> {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
        self.type_layout(type_name, &mut Vec::new())
    }

    fn enum_layout_for(
        &self,
        type_name: &TypeName,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        let normalized = self.normalize_type_name(type_name);
        let type_name = &normalized;
        if let Some(layout) = self.niche_option_layout(type_name) {
            return Some(layout);
        }
        if type_name.name == "Option" && type_name.arguments.len() == 1 {
            return self.generic_option_layout(type_name);
        }
        if type_name.name == "Result" && type_name.arguments.len() == 2 {
            return self.result_layout(type_name);
        }
        self.named_enum_layout(type_name, &mut Vec::new())
    }

    fn enum_variant(&self, type_name: &TypeName, variant: &str) -> Option<(u32, bool)> {
        let normalized = self.normalize_type_name(type_name);
        let type_name = &normalized;
        if let Some(variant) = niche_option_variant(type_name, variant) {
            return Some(variant);
        }
        if type_name.name == "Option" && type_name.arguments.len() == 1 {
            return match variant {
                "Some" => Some((0, false)),
                "None" => Some((1, true)),
                _ => None,
            };
        }
        let definition = self.enums.get(type_name.name.as_str())?;
        let (index, definition) = definition
            .variants
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.name == variant)?;
        Some((index as u32, matches!(definition.payload, EnumPayload::Unit)))
    }

    fn enum_variant_payload_fields(
        &self,
        type_name: &TypeName,
        variant: &str,
    ) -> Option<Vec<super::super::BackendEnumPayloadField>> {
        let normalized = self.normalize_type_name(type_name);
        if let Some(fields) = builtin_enum_payload_fields(self, &normalized, variant) {
            return fields;
        }
        named_enum_payload_fields(self, &normalized, variant)
    }

    fn enum_variant_for_layout(
        &self,
        layout: super::super::abi::BackendEnumLayout,
        variant: &str,
    ) -> Option<(u32, bool)> {
        if layout.niche_pointer {
            return niche_pointer_variant(variant);
        }
        self.enums.values().find_map(|definition| {
            let type_name = TypeName {
                name: definition.name.clone(),
                arguments: Vec::new(),
                reference_role: None,
                span: definition.span,
            };
            (self.enum_layout_for(&type_name) == Some(layout))
                .then(|| self.enum_variant(&type_name, variant))
                .flatten()
        })
    }

    fn type_for_layout(&self, layout: BackendLayout) -> Option<TypeName> {
        self.packs
            .values()
            .find_map(|definition| {
                let type_name = TypeName {
                    name: definition.name.clone(),
                    arguments: Vec::new(),
                    reference_role: None,
                    span: definition.span,
                };
                (self.layout_for(&type_name) == Some(layout)).then_some(type_name)
            })
            .or_else(|| {
                self.structs.values().find_map(|definition| {
                    let type_name = TypeName {
                        name: definition.name.clone(),
                        arguments: Vec::new(),
                        reference_role: None,
                        span: definition.span,
                    };
                    (self.layout_for(&type_name) == Some(layout)).then_some(type_name)
                })
            })
    }

    fn packed_field_for(
        &self,
        type_name: &TypeName,
        field_name: &str,
    ) -> Option<super::super::BackendPackedFieldLayout> {
        let pack = self.packs.get(type_name.name.as_str())?;
        let field = pack.fields.iter().find(|field| field.name == field_name)?;
        let primitive = if field.ty.name == "Array" && field.ty.arguments.len() == 2 {
            &field.ty.arguments[0]
        } else {
            &field.ty
        };
        let width = match primitive_type(&primitive.name)? {
            crate::ast::PrimitiveType::Integer { width, .. } => width,
            _ => return None,
        };
        let scalar_ty = match primitive_type(&primitive.name)? {
            crate::ast::PrimitiveType::Integer { signed, width } => {
                let width = match width {
                    1..=8 => 8,
                    9..=16 => 16,
                    17..=32 => 32,
                    33..=64 => 64,
                    _ => return None,
                };
                super::super::BackendValueType::Integer { signed, width }
            }
            _ => return None,
        };
        let storage = self.type_layout(pack.storage.type_name(), &mut Vec::new())?;
        Some(super::super::BackendPackedFieldLayout {
            storage,
            bit_offset: field.offset,
            width,
            big_endian: matches!(pack.endianness, crate::ast::LayoutEndianness::Big),
            scalar_ty,
            ty: field.ty.clone(),
        })
    }

    fn field_for(&self, type_name: &TypeName, field_name: &str) -> Option<BackendFieldLayout> {
        let normalized = self.normalize_type_name(type_name);
        let type_name = &normalized;
        if self.packs.contains_key(type_name.name.as_str()) {
            return self.pack_field_for(type_name, field_name);
        }
        let definition = self.structs.get(type_name.name.as_str())?;
        let packed = definition
            .repr()
            .is_some_and(|repr| repr.is_packed() || repr.is_transparent());
        let substitution = (!definition.generic_parameters.is_empty())
            .then(|| {
                crate::semantic::TypeSubstitution::for_type(
                    &definition.name,
                    &definition.generic_parameters,
                    &type_name.arguments,
                    definition.span,
                )
                .ok()
            })
            .flatten();
        let mut offset = 0u32;
        for field in &definition.fields {
            let field_type = substitution
                .as_ref()
                .map_or_else(|| field.ty.clone(), |substitution| substitution.apply(&field.ty));
            let layout = self.field_storage_layout(&field.role, &field_type)?;
            let field_alignment = if packed { 1 } else { layout.alignment };
            offset = Self::align_up(offset, field_alignment)?;
            if field.name == field_name {
                return Some(BackendFieldLayout {
                    offset,
                    layout,
                    ty: field_type,
                    indirect: matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins),
                });
            }
            offset = offset.checked_add(layout.size)?;
        }
        None
    }

}

fn builtin_enum_payload_fields(
    layouts: &ProgramLayouts<'_>,
    type_name: &TypeName,
    variant: &str,
) -> Option<Option<Vec<super::super::BackendEnumPayloadField>>> {
    let index = match type_name.name.as_str() {
        "Option" if type_name.arguments.len() == 1 => match variant {
            "None" => return Some(Some(Vec::new())),
            "Some" => 0,
            _ => return Some(None),
        },
        "Result" if type_name.arguments.len() == 2 => match variant {
            "Ok" => 0,
            "Err" => 1,
            _ => return Some(None),
        },
        _ => return None,
    };
    let ty = type_name.arguments[index].clone();
    let layout = layouts.type_layout(&ty, &mut Vec::new())?;
    Some(Some(vec![super::super::BackendEnumPayloadField {
        name: "0".to_owned(),
        offset: 0,
        layout,
        ty,
    }]))
}

fn named_enum_payload_fields(
    layouts: &ProgramLayouts<'_>,
    type_name: &TypeName,
    variant: &str,
) -> Option<Vec<super::super::BackendEnumPayloadField>> {
    let definition = layouts.enums.get(type_name.name.as_str())?;
    let variant = definition.variants.iter().find(|candidate| candidate.name == variant)?;
    let substitution = (!definition.generic_parameters.is_empty())
        .then(|| {
            crate::semantic::TypeSubstitution::for_type(
                &definition.name,
                &definition.generic_parameters,
                &type_name.arguments,
                definition.span,
            )
            .ok()
        })
        .flatten();
    let fields = match &variant.payload {
        EnumPayload::Unit => return Some(Vec::new()),
        EnumPayload::Tuple(types) => types
            .iter()
            .enumerate()
            .map(|(index, ty)| (index.to_string(), ty.clone()))
            .collect::<Vec<_>>(),
        EnumPayload::Struct(fields) => fields
            .iter()
            .map(|field| (field.name.clone(), field.ty.clone()))
            .collect::<Vec<_>>(),
    };
    let mut offset = 0u32;
    let mut result = Vec::new();
    for (name, ty) in fields {
        let ty = substitution
            .as_ref()
            .map_or_else(|| ty.clone(), |substitution| substitution.apply(&ty));
        let layout = layouts.type_layout(&ty, &mut Vec::new())?;
        offset = ProgramLayouts::align_up(offset, layout.alignment)?;
        result.push(super::super::BackendEnumPayloadField { name, offset, layout, ty });
        offset = offset.checked_add(layout.size)?;
    }
    Some(result)
}

impl ProgramLayouts<'_> {
    fn pack_field_for(&self, type_name: &TypeName, field_name: &str) -> Option<BackendFieldLayout> {
        let pack = self.packs.get(type_name.name.as_str())?;
        if field_name == "storage" {
            let storage = pack.storage.type_name().clone();
            let layout = self.type_layout(&storage, &mut Vec::new())?;
            return Some(BackendFieldLayout { offset: 0, layout, ty: storage, indirect: false });
        }
        let field = pack.fields.iter().find(|field| field.name == field_name)?;
        if field.ty.name != "Array"
            || field.ty.arguments.len() != 2
            || !field.offset.is_multiple_of(8)
        {
            return None;
        }
        let layout = self.type_layout(&field.ty, &mut Vec::new())?;
        Some(BackendFieldLayout {
            offset: u32::from(field.offset / 8),
            layout,
            ty: field.ty.clone(),
            indirect: false,
        })
    }

    fn result_layout(
        &self,
        type_name: &TypeName,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        let payload = type_name
            .arguments
            .iter()
            .filter(|argument| argument.name != "Void")
            .filter_map(|argument| self.type_layout(argument, &mut Vec::new()))
            .max_by_key(|layout| (layout.size, layout.alignment));
        let payload_offset = match payload.as_ref() {
            Some(layout) => Self::align_up(4, layout.alignment)?,
            None => 4,
        };
        let alignment = payload.as_ref().map_or(4, |layout| 4u32.max(layout.alignment));
        Some(super::super::abi::BackendEnumLayout {
            layout: BackendLayout {
                size: Self::align_up(
                    payload_offset.checked_add(payload.as_ref().map_or(0, |layout| layout.size))?,
                    alignment,
                )?,
                alignment,
            },
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset,
            payload,
            niche_pointer: false,
        })
    }
}

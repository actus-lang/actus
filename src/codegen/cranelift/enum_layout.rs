use crate::ast::{EnumDef, EnumPayload, TypeName};

use super::layout::{LayoutRegistry, integer_storage_bytes};
use super::native::NativeEmitError;
use super::types::NativeType;

#[derive(Clone, Debug)]
pub(super) struct EnumFieldLayout {
    pub(super) name: Option<String>,
    pub(super) offset: u32,
    pub(super) ty: NativeType,
}

#[derive(Clone, Debug)]
pub(super) struct EnumVariantLayout {
    pub(super) name: String,
    pub(super) discriminant: u32,
    pub(super) fields: Vec<EnumFieldLayout>,
}

#[derive(Clone, Debug)]
pub(super) struct EnumLayout {
    pub(super) size: u32,
    pub(super) alignment: u32,
    pub(super) discriminant_offset: u32,
    pub(super) payload_offset: u32,
    pub(super) niche_pointer: bool,
    pub(super) variants: Vec<EnumVariantLayout>,
}

impl LayoutRegistry {
    pub(super) fn enum_id_for(&self, name: &str) -> Option<usize> {
        self.enum_ids.get(name).copied()
    }

    pub(super) fn enum_layout(&self, id: usize) -> Option<&EnumLayout> {
        self.enum_layouts.get(id)
    }

    pub(super) fn is_niche_option(&self, id: usize) -> bool {
        self.enum_layout(id).is_some_and(|layout| layout.niche_pointer)
    }

    pub(super) fn enum_constructor(
        &self,
        receiver: &str,
        variant: &str,
    ) -> Option<(usize, &EnumVariantLayout)> {
        let id = self.enum_id_for(receiver)?;
        let layout = self.enum_layout(id)?;
        layout
            .variants
            .iter()
            .find(|candidate| candidate.name == variant)
            .map(|variant| (id, variant))
    }

    pub(super) fn enum_variant(&self, enum_id: usize, variant: &str) -> Option<&EnumVariantLayout> {
        self.enum_layout(enum_id)?.variants.iter().find(|candidate| candidate.name == variant)
    }

    pub(super) fn type_size(&self, ty: NativeType) -> Option<u32> {
        match ty {
            NativeType::Struct(id) => self.get(id).map(|layout| layout.size),
            NativeType::Enum(id) => self.enum_layout(id).map(|layout| layout.size),
            NativeType::Int => Some(4),
            NativeType::Integer { width, .. } => integer_storage_bytes(width),
            NativeType::Float { width } => Some(u32::from(width / 8)),
            NativeType::Void => Some(0),
            NativeType::String | NativeType::Buffer => Some(self.pointer_size),
            NativeType::Region => Some(self.pointer_size),
            NativeType::FatPointer => Some(self.pointer_size * 2),
            NativeType::Pack(id) => self.pack_size(id),
            NativeType::Array(id) => self.array_layout_value(id).map(|array| array.size),
            NativeType::Arena(capacity) => Some(capacity + self.pointer_size),
        }
    }

    pub(super) fn is_trivially_copyable_struct(&self, id: usize) -> bool {
        self.get(id).is_some_and(|layout| {
            layout.fields.iter().all(|field| self.is_trivially_copyable(field.ty))
        })
    }

    fn is_trivially_copyable(&self, ty: NativeType) -> bool {
        match ty {
            NativeType::Int
            | NativeType::Integer { .. }
            | NativeType::Float { .. }
            | NativeType::Void
            | NativeType::Pack(_) => true,
            NativeType::Struct(id) => self.is_trivially_copyable_struct(id),
            NativeType::Array(id) => self
                .array_layout_value(id)
                .is_some_and(|layout| self.is_trivially_copyable(layout.element)),
            NativeType::String
            | NativeType::Buffer
            | NativeType::Arena(_)
            | NativeType::Region
            | NativeType::Enum(_)
            | NativeType::FatPointer => false,
        }
    }

    fn pack_size(&self, id: usize) -> Option<u32> {
        self.pack_definitions
            .get(id)
            .and_then(|pack| pack.storage.byte_capacity())
            .and_then(|size| u32::try_from(size).ok())
            .or_else(|| self.pack(id).and_then(|pack| self.type_size(pack.storage)))
    }

    pub(super) fn enum_layout_for(
        &self,
        definition: &EnumDef,
        visiting: &mut Vec<String>,
    ) -> Result<EnumLayout, NativeEmitError> {
        if visiting.contains(&definition.name) {
            return Err(NativeEmitError(format!(
                "recursive enum layout for `{}` is not supported",
                definition.name
            )));
        }
        visiting.push(definition.name.clone());
        let niche_pointer = is_pointer_option(definition);
        let (variants, max_payload_size, max_payload_alignment) =
            self.enum_variants(definition, visiting)?;
        visiting.pop();
        if niche_pointer {
            return Ok(self.niche_enum_layout(variants));
        }
        Ok(regular_enum_layout(variants, max_payload_size, max_payload_alignment))
    }

    fn niche_enum_layout(&self, variants: Vec<EnumVariantLayout>) -> EnumLayout {
        EnumLayout {
            size: self.pointer_size,
            alignment: self.pointer_size,
            discriminant_offset: 0,
            payload_offset: 0,
            niche_pointer: true,
            variants,
        }
    }

    fn enum_variants(
        &self,
        definition: &EnumDef,
        visiting: &mut Vec<String>,
    ) -> Result<(Vec<EnumVariantLayout>, u32, u32), NativeEmitError> {
        let mut variants = Vec::new();
        let mut max_payload_size = 0;
        let mut max_payload_alignment = 1;
        for (discriminant, variant) in definition.variants.iter().enumerate() {
            let (fields, payload_size, payload_alignment) =
                self.enum_variant_fields(&variant.payload, visiting)?;
            max_payload_size = max_payload_size.max(payload_size);
            max_payload_alignment = max_payload_alignment.max(payload_alignment);
            variants.push(EnumVariantLayout {
                name: variant.name.clone(),
                discriminant: u32::try_from(discriminant)
                    .map_err(|_| NativeEmitError("enum has too many variants".to_owned()))?,
                fields,
            });
        }
        Ok((variants, max_payload_size, max_payload_alignment))
    }

    fn enum_variant_fields(
        &self,
        payload: &EnumPayload,
        visiting: &mut Vec<String>,
    ) -> Result<(Vec<EnumFieldLayout>, u32, u32), NativeEmitError> {
        let mut fields = Vec::new();
        let mut payload_size = 0;
        let mut payload_alignment = 1;
        for (name, type_name) in enum_payload_types(payload) {
            let ty = self.native_type_for_type_name(type_name, visiting)?;
            let (size, alignment) = if type_name.reference_role.is_some() {
                (self.pointer_size, self.pointer_size)
            } else {
                self.type_layout(ty)?
            };
            payload_size = align_up(payload_size, alignment);
            fields.push(EnumFieldLayout { name, offset: payload_size, ty });
            payload_size += size;
            payload_alignment = payload_alignment.max(alignment);
        }
        Ok((fields, align_up(payload_size, payload_alignment), payload_alignment))
    }
}

fn regular_enum_layout(
    variants: Vec<EnumVariantLayout>,
    max_payload_size: u32,
    max_payload_alignment: u32,
) -> EnumLayout {
    let discriminant_size = 4;
    let payload_offset = align_up(discriminant_size, max_payload_alignment);
    let alignment = discriminant_size.max(max_payload_alignment);
    EnumLayout {
        size: align_up(payload_offset + max_payload_size, alignment),
        alignment,
        discriminant_offset: 0,
        payload_offset,
        niche_pointer: false,
        variants,
    }
}

fn enum_payload_types(payload: &EnumPayload) -> Vec<(Option<String>, &TypeName)> {
    match payload {
        EnumPayload::Unit => Vec::new(),
        EnumPayload::Tuple(types) => types.iter().map(|ty| (None, ty)).collect(),
        EnumPayload::Struct(fields) => {
            fields.iter().map(|field| (Some(field.name.clone()), &field.ty)).collect()
        }
    }
}

fn is_pointer_option(definition: &EnumDef) -> bool {
    if !definition.name.starts_with("Option[") {
        return false;
    }
    let Some(payload) =
        definition.variants.iter().find(|variant| variant.name == "Some").and_then(|variant| {
            match &variant.payload {
                EnumPayload::Tuple(types) if types.len() == 1 => types.first(),
                _ => None,
            }
        })
    else {
        return false;
    };
    definition.variants.iter().any(|variant| variant.name == "None")
        && matches!(payload.reference_role, Some(crate::ast::Role::Abs | crate::ast::Role::Ins))
}

fn align_up(offset: u32, alignment: u32) -> u32 {
    offset.div_ceil(alignment) * alignment
}

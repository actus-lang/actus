use cranelift_codegen::ir::{StackSlotData, StackSlotKind};

use crate::ast::{EnumDef, EnumPayload};

use super::layout::LayoutRegistry;
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
    pub(super) max_payload_size: u32,
    pub(super) variants: Vec<EnumVariantLayout>,
}

impl LayoutRegistry {
    pub(super) fn enum_id_for(&self, name: &str) -> Option<usize> {
        self.enum_ids.get(name).copied()
    }

    pub(super) fn enum_layout(&self, id: usize) -> Option<&EnumLayout> {
        self.enum_layouts.get(id)
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

    pub(super) fn enum_stack_slot(&self, layout: &EnumLayout) -> StackSlotData {
        StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            layout.size.max(layout.payload_offset + layout.max_payload_size),
            layout.alignment.trailing_zeros() as u8,
        )
    }

    pub(super) fn type_size(&self, ty: NativeType) -> Option<u32> {
        match ty {
            NativeType::Struct(id) => self.get(id).map(|layout| layout.size),
            NativeType::Enum(id) => self.enum_layout(id).map(|layout| layout.size),
            NativeType::Int => Some(4),
            NativeType::String | NativeType::Buffer => Some(self.pointer_size),
        }
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
        let mut variants = Vec::new();
        let mut max_payload_size = 0;
        let mut max_payload_alignment = 1;
        for (discriminant, variant) in definition.variants.iter().enumerate() {
            let mut fields = Vec::new();
            let mut payload_size = 0;
            let mut payload_alignment = 1;
            for (name, type_name) in enum_payload_types(&variant.payload) {
                let ty = self.native_type(type_name, visiting)?;
                let (size, alignment) = self.type_layout(ty)?;
                payload_size = align_up(payload_size, alignment);
                fields.push(EnumFieldLayout { name, offset: payload_size, ty });
                payload_size += size;
                payload_alignment = payload_alignment.max(alignment);
            }
            payload_size = align_up(payload_size, payload_alignment);
            max_payload_size = max_payload_size.max(payload_size);
            max_payload_alignment = max_payload_alignment.max(payload_alignment);
            variants.push(EnumVariantLayout {
                name: variant.name.clone(),
                discriminant: u32::try_from(discriminant)
                    .map_err(|_| NativeEmitError("enum has too many variants".to_owned()))?,
                fields,
            });
        }
        visiting.pop();
        let discriminant_size = 4;
        let payload_offset = align_up(discriminant_size, max_payload_alignment);
        let alignment = discriminant_size.max(max_payload_alignment);
        Ok(EnumLayout {
            size: align_up(payload_offset + max_payload_size, alignment),
            alignment,
            discriminant_offset: 0,
            payload_offset,
            max_payload_size,
            variants,
        })
    }
}

fn enum_payload_types(payload: &EnumPayload) -> Vec<(Option<String>, &str)> {
    match payload {
        EnumPayload::Unit => Vec::new(),
        EnumPayload::Tuple(types) => types.iter().map(|ty| (None, ty.name.as_str())).collect(),
        EnumPayload::Struct(fields) => {
            fields.iter().map(|field| (Some(field.name.clone()), field.ty.name.as_str())).collect()
        }
    }
}

fn align_up(offset: u32, alignment: u32) -> u32 {
    offset.div_ceil(alignment) * alignment
}

#[cfg(test)]
mod tests {
    use super::LayoutRegistry;
    use crate::lexer::scan;
    use crate::parser::parse;
    use cranelift_codegen::ir::types;

    #[test]
    fn calculates_enum_discriminant_and_payload_alignment() {
        let (tokens, errors) = scan(
            "enum Value { Empty, Count(Int), Text(String), Pair { left: Int, right: Int, }, }",
        );
        assert!(errors.is_empty());
        let program = parse(tokens).expect("enum should parse");
        let layouts =
            LayoutRegistry::from_program(&program, types::I64).expect("enum layout should pass");
        let layout = layouts
            .enum_layout(layouts.enum_id_for("Value").expect("Value layout should exist"))
            .unwrap();

        assert_eq!(layout.discriminant_offset, 0);
        assert_eq!(layout.payload_offset, 8);
        assert_eq!(layout.max_payload_size, 8);
        assert_eq!(layout.alignment, 8);
        assert_eq!(layout.size, 16);
        assert_eq!(layout.variants[0].discriminant, 0);
        assert_eq!(layout.variants[1].fields[0].offset, 0);
        assert_eq!(layout.variants[3].fields[1].offset, 4);
    }

    #[test]
    fn calculates_unit_enum_layout_without_payload_padding() {
        let (tokens, errors) = scan("enum Color { Red, Green, }");
        assert!(errors.is_empty());
        let program = parse(tokens).expect("unit enum should parse");
        let layouts =
            LayoutRegistry::from_program(&program, types::I64).expect("enum layout should pass");
        let layout = layouts
            .enum_layout(layouts.enum_id_for("Color").expect("Color layout should exist"))
            .unwrap();

        assert_eq!(layout.size, 4);
        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.max_payload_size, 0);
    }
}

use crate::ast::{EnumDef, EnumPayload, TypeName};
use crate::semantic::TypeSubstitution;

use super::generic_layout::{
    GenericEnumLayout, GenericEnumVariantLayout, GenericFieldLayout, GenericLayoutRegistry,
    align_up, application_key, enter_layout,
};
use super::native::NativeEmitError;

impl GenericLayoutRegistry {
    pub(super) fn layout_enum(
        &self,
        definition: &EnumDef,
        arguments: &[TypeName],
        visiting: &mut Vec<String>,
    ) -> Result<GenericEnumLayout, NativeEmitError> {
        let canonical_key = application_key(&definition.name, arguments);
        enter_layout(&canonical_key, visiting)?;
        let substitution = TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            arguments,
            definition.span,
        )
        .map_err(|error| NativeEmitError(format!("generic substitution failed: {error:?}")))?;
        let result = self.layout_enum_variants(definition, &substitution, visiting);
        visiting.pop();
        result.map(|(size, alignment, payload_offset, max_payload_size, variants)| {
            GenericEnumLayout {
                canonical_key,
                size,
                alignment,
                payload_offset,
                max_payload_size,
                variants,
            }
        })
    }

    fn layout_enum_variants(
        &self,
        definition: &EnumDef,
        substitution: &TypeSubstitution,
        visiting: &mut Vec<String>,
    ) -> Result<(u32, u32, u32, u32, Vec<GenericEnumVariantLayout>), NativeEmitError> {
        let mut variants = Vec::new();
        let mut max_payload_size = 0;
        let mut max_payload_alignment = 1;
        for (index, variant) in definition.variants.iter().enumerate() {
            let mut fields = Vec::new();
            let mut offset = 0;
            let mut payload_alignment = 1;
            for (name, field_type, owned) in variant_fields(&variant.payload) {
                let layout = self.layout_type(&substitution.apply(field_type), visiting)?;
                offset = align_up(offset, layout.alignment);
                fields.push(GenericFieldLayout {
                    name,
                    offset,
                    size: layout.size,
                    alignment: layout.alignment,
                    owned,
                });
                offset += layout.size;
                payload_alignment = payload_alignment.max(layout.alignment);
            }
            let payload_size = align_up(offset, payload_alignment);
            max_payload_size = max_payload_size.max(payload_size);
            max_payload_alignment = max_payload_alignment.max(payload_alignment);
            variants.push(GenericEnumVariantLayout {
                name: variant.name.clone(),
                discriminant: u32::try_from(index)
                    .map_err(|_| NativeEmitError("enum has too many variants".to_owned()))?,
                fields,
            });
        }
        let payload_offset = align_up(4, max_payload_alignment);
        let alignment = 4.max(max_payload_alignment);
        let size = align_up(payload_offset + max_payload_size, alignment);
        Ok((size, alignment, payload_offset, max_payload_size, variants))
    }
}

fn variant_fields(payload: &EnumPayload) -> Vec<(Option<String>, &TypeName, bool)> {
    match payload {
        EnumPayload::Unit => Vec::new(),
        EnumPayload::Tuple(types) => types.iter().map(|ty| (None, ty, false)).collect(),
        EnumPayload::Struct(fields) => {
            fields.iter().map(|field| (Some(field.name.clone()), &field.ty, false)).collect()
        }
    }
}

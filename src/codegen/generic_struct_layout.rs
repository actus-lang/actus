use crate::ast::{StructDef, StructFieldRole, TypeName};
use crate::semantic::TypeSubstitution;

use super::generic_layout::{
    GenericFieldLayout, GenericLayoutRegistry, GenericStructLayout, align_up, application_key,
    enter_layout,
};
use super::native::NativeEmitError;

impl GenericLayoutRegistry {
    pub(super) fn layout_struct(
        &self,
        definition: &StructDef,
        arguments: &[TypeName],
        visiting: &mut Vec<String>,
    ) -> Result<GenericStructLayout, NativeEmitError> {
        let canonical_key = application_key(&definition.name, arguments);
        enter_layout(&canonical_key, visiting)?;
        let substitution = TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            arguments,
            definition.span,
        )
        .map_err(|error| NativeEmitError(format!("generic substitution failed: {error:?}")))?;
        let result = self.layout_struct_fields(definition, &substitution, visiting);
        visiting.pop();
        result.map(|(size, alignment, fields)| GenericStructLayout {
            canonical_key,
            size,
            alignment,
            fields,
        })
    }

    fn layout_struct_fields(
        &self,
        definition: &StructDef,
        substitution: &TypeSubstitution,
        visiting: &mut Vec<String>,
    ) -> Result<(u32, u32, Vec<GenericFieldLayout>), NativeEmitError> {
        let mut fields = Vec::new();
        let mut offset = 0;
        let mut alignment = 1;
        for field in &definition.fields {
            let layout = self.layout_type(&substitution.apply(&field.ty), visiting)?;
            offset = align_up(offset, layout.alignment);
            fields.push(GenericFieldLayout {
                name: Some(field.name.clone()),
                offset,
                size: layout.size,
                alignment: layout.alignment,
                owned: matches!(field.role, StructFieldRole::Erg),
            });
            offset += layout.size;
            alignment = alignment.max(layout.alignment);
        }
        Ok((align_up(offset, alignment), alignment, fields))
    }
}

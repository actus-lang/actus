use crate::ast::TypeName;
use crate::semantic::TypeSubstitution;

use super::super::super::BackendLayout;
use super::ProgramLayouts;

impl ProgramLayouts<'_> {
    pub(super) fn generic_struct_layout(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Option<BackendLayout> {
        let definition = self.structs.get(type_name.name.as_str())?;
        let key = type_name.canonical_key();
        if visiting.iter().any(|candidate| candidate == &key) {
            return None;
        }
        let substitution = TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            &type_name.arguments,
            definition.span,
        )
        .ok()?;
        visiting.push(key);
        let mut size = 0u32;
        let mut alignment = 1u32;
        for field in &definition.fields {
            let field_type = substitution.apply(&field.ty);
            let field_layout = self.field_storage_layout(&field.role, &field_type)?;
            size = Self::align_up(size, field_layout.alignment)?;
            size = size.checked_add(field_layout.size)?;
            alignment = alignment.max(field_layout.alignment);
        }
        visiting.pop();
        Some(BackendLayout { size: Self::align_up(size, alignment)?, alignment })
    }
}

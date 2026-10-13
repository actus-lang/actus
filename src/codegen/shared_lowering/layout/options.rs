use crate::ast::Role;

impl ProgramLayouts<'_> {
    fn niche_option_layout(
        &self,
        type_name: &TypeName,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        if !is_niche_pointer_option(type_name) {
            return None;
        }
        Some(super::super::abi::BackendEnumLayout {
            layout: self.pointer_layout(),
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset: 0,
            payload: None,
            niche_pointer: true,
        })
    }

    fn generic_option_layout(
        &self,
        type_name: &TypeName,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        let payload = self.type_layout(&type_name.arguments[0], &mut Vec::new())?;
        let payload_offset = Self::align_up(4, payload.alignment)?;
        let alignment = 4u32.max(payload.alignment);
        Some(super::super::abi::BackendEnumLayout {
            layout: BackendLayout {
                size: Self::align_up(payload_offset.checked_add(payload.size)?, alignment)?,
                alignment,
            },
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset,
            payload: Some(payload),
            niche_pointer: false,
        })
    }
}

fn is_niche_pointer_option(type_name: &TypeName) -> bool {
    type_name.name == "Option"
        && type_name.arguments.len() == 1
        && matches!(type_name.arguments[0].reference_role, Some(Role::Abs | Role::Ins))
}

fn niche_option_variant(type_name: &TypeName, variant: &str) -> Option<(u32, bool)> {
    is_niche_pointer_option(type_name).then(|| niche_pointer_variant(variant)).flatten()
}

fn niche_pointer_variant(variant: &str) -> Option<(u32, bool)> {
    match variant {
        "None" => Some((0, true)),
        "Some" => Some((1, false)),
        _ => None,
    }
}

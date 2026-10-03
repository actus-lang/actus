use crate::ast::{PackDecl, PackField, PrimitiveType, Role, StructFieldRole, primitive_type};

use super::{LayoutRegistry, PackFieldLayout, PackLayout};
use crate::codegen::native::NativeEmitError;

impl LayoutRegistry {
    pub(super) fn pack_layout_for(&self, pack: &PackDecl) -> Result<PackLayout, NativeEmitError> {
        let storage = self.type_for_type_name(pack.storage.type_name()).ok_or_else(|| {
            NativeEmitError(format!("unknown packed storage `{}`", pack.storage.canonical_key()))
        })?;
        let fields = pack
            .fields
            .iter()
            .map(|field| self.pack_field_layout(field))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PackLayout { storage, endianness: pack.endianness, fields })
    }

    fn pack_field_layout(&self, field: &PackField) -> Result<PackFieldLayout, NativeEmitError> {
        let width = packed_field_width(field)?;
        let role = match field.role {
            Role::Erg => StructFieldRole::Erg,
            Role::Abs => StructFieldRole::Value,
            Role::Dat | Role::Ins => {
                return Err(NativeEmitError(format!(
                    "invalid packed field role for `{}`",
                    field.name
                )));
            }
        };
        Ok(PackFieldLayout {
            name: field.name.clone(),
            role,
            ty: self.native_type(&field.ty.name, &mut Vec::new())?,
            offset: field.offset,
            width,
        })
    }
}

fn packed_field_width(field: &PackField) -> Result<u8, NativeEmitError> {
    primitive_type(&field.ty.name)
        .and_then(|primitive| match primitive {
            PrimitiveType::Integer { width, .. } => Some(width),
            _ => None,
        })
        .ok_or_else(|| NativeEmitError(format!("invalid packed field `{}`", field.name)))
}

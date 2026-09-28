use crate::ast::{
    BuiltinType, StructDef, StructFieldRole, TypeName, lookup_builtin_type, primitive_type,
};

use super::{FieldLayout, LayoutRegistry, StructLayout, align_up, integer_storage_bytes};
use crate::codegen::native::NativeEmitError;
use crate::codegen::types::NativeType;

impl LayoutRegistry {
    pub(in crate::codegen) fn layout_for(
        &self,
        definition: &StructDef,
        visiting: &mut Vec<String>,
    ) -> Result<StructLayout, NativeEmitError> {
        if visiting.contains(&definition.name) {
            return Err(NativeEmitError(format!(
                "recursive struct layout for `{}` is not supported",
                definition.name
            )));
        }
        visiting.push(definition.name.clone());
        let mut fields = Vec::new();
        let mut offset = 0;
        let mut alignment = 1;
        for field in &definition.fields {
            let indirect = matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins);
            let ty = if indirect {
                self.type_for_type_name(&field.ty).ok_or_else(|| {
                    NativeEmitError(format!("unknown reference field type `{}`", field.ty.name))
                })?
            } else {
                self.native_type_for_type_name(&field.ty, visiting)?
            };
            let (size, field_alignment) = if indirect {
                (self.pointer_size, self.pointer_size)
            } else {
                self.type_layout(ty)?
            };
            offset = align_up(offset, field_alignment);
            fields.push(FieldLayout {
                name: field.name.clone(),
                offset,
                ty,
                owned: matches!(field.role, StructFieldRole::Erg),
                indirect,
            });
            offset += size;
            alignment = alignment.max(field_alignment);
        }
        visiting.pop();
        Ok(StructLayout { size: align_up(offset, alignment), alignment, fields })
    }

    pub(in crate::codegen) fn native_type(
        &self,
        name: &str,
        visiting: &mut Vec<String>,
    ) -> Result<NativeType, NativeEmitError> {
        if let Some(primitive) = primitive_type(name) {
            return Ok(NativeType::from_primitive(primitive));
        }
        if let Some(ty) = lookup_builtin_type(name) {
            return match ty {
                BuiltinType::Int => Ok(NativeType::Int),
                BuiltinType::Bool => Ok(NativeType::Int),
                BuiltinType::String => Ok(NativeType::String),
                BuiltinType::Buffer => Ok(NativeType::Buffer),
                BuiltinType::Array | BuiltinType::Map => {
                    Err(NativeEmitError(format!("unsupported layout type `{name}`")))
                }
            };
        }
        if let Some(enum_id) = self.enum_id_for(name) {
            let definition = self
                .enum_definitions
                .get(enum_id)
                .ok_or_else(|| NativeEmitError(format!("missing enum layout type `{name}`")))?;
            self.enum_layout_for(definition, visiting)?;
            return Ok(NativeType::Enum(enum_id));
        }
        if let Some(pack_id) = self.pack_ids.get(name).copied() {
            return Ok(NativeType::Pack(pack_id));
        }
        let id = self
            .id_for(name)
            .ok_or_else(|| NativeEmitError(format!("unknown layout type `{name}`")))?;
        if visiting.iter().any(|candidate| candidate == name) {
            return Err(NativeEmitError(format!(
                "recursive struct layout for `{name}` is not supported"
            )));
        }
        let definition = self
            .definitions
            .get(id)
            .ok_or_else(|| NativeEmitError(format!("missing layout type `{name}`")));
        self.layout_for(definition?, visiting)?;
        Ok(NativeType::Struct(id))
    }

    pub(in crate::codegen) fn native_type_for_type_name(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Result<NativeType, NativeEmitError> {
        if let Some(native_type) = self.type_for_type_name(type_name) {
            return Ok(native_type);
        }
        self.native_type(&type_name.name, visiting)
    }

    pub(in crate::codegen) fn type_layout(
        &self,
        ty: NativeType,
    ) -> Result<(u32, u32), NativeEmitError> {
        Ok(match ty {
            NativeType::Int => (4, 4),
            NativeType::Integer { width, .. } => {
                let bytes = integer_storage_bytes(width);
                (bytes, bytes)
            }
            NativeType::Float { width } => {
                let bytes = u32::from(width / 8);
                (bytes, bytes)
            }
            NativeType::Void => (0, 1),
            NativeType::String | NativeType::Buffer => (self.pointer_size, self.pointer_size),
            NativeType::FatPointer => (self.pointer_size * 2, self.pointer_size),
            NativeType::Struct(id) => {
                let definition = self.definitions.get(id).ok_or_else(|| {
                    NativeEmitError(format!("missing nested struct layout `{id}"))
                })?;
                let layout = self.layout_for(definition, &mut Vec::new())?;
                (layout.size, layout.alignment)
            }
            NativeType::Enum(id) => {
                let layout = if let Some(layout) = self.enum_layout(id) {
                    layout.clone()
                } else {
                    let definition = self
                        .enum_definitions
                        .get(id)
                        .ok_or_else(|| NativeEmitError(format!("missing enum layout `{id}")))?;
                    self.enum_layout_for(definition, &mut Vec::new())?
                };
                (layout.size, layout.alignment)
            }
            NativeType::Pack(id) => self
                .pack(id)
                .map(|pack| self.type_layout(pack.storage))
                .transpose()?
                .ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?,
            NativeType::Arena(capacity) => (capacity + self.pointer_size, self.pointer_size),
        })
    }

    pub(in crate::codegen) fn alignment(&self, ty: NativeType) -> Result<u32, NativeEmitError> {
        Ok(match ty {
            NativeType::Struct(id) => self
                .get(id)
                .map(|layout| layout.alignment)
                .ok_or_else(|| NativeEmitError("missing struct alignment".to_owned()))?,
            NativeType::Enum(id) => self
                .enum_layout(id)
                .map(|layout| layout.alignment)
                .ok_or_else(|| NativeEmitError("missing enum alignment".to_owned()))?,
            NativeType::Integer { width, .. } => integer_storage_bytes(width),
            NativeType::Float { width } => u32::from(width / 8),
            NativeType::Int => 4,
            NativeType::String | NativeType::Buffer | NativeType::Arena(_) => self.pointer_size,
            NativeType::FatPointer => self.pointer_size,
            NativeType::Void => 1,
            NativeType::Pack(id) => {
                self.pack(id).map(|pack| self.alignment(pack.storage)).transpose()?.unwrap_or(1)
            }
        })
    }
}

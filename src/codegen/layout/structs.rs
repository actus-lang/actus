use crate::ast::{
    BuiltinType, StructDef, StructField, StructFieldRole, TypeName, lookup_builtin_type,
    primitive_type,
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
            let (mut field_layout, size, field_alignment) = self.field_layout(field, visiting)?;
            offset = align_up(offset, field_alignment);
            field_layout.offset = offset;
            fields.push(field_layout);
            offset += size;
            alignment = alignment.max(field_alignment);
        }
        visiting.pop();
        Ok(StructLayout { size: align_up(offset, alignment), alignment, fields })
    }

    fn field_layout(
        &self,
        field: &StructField,
        visiting: &mut Vec<String>,
    ) -> Result<(FieldLayout, u32, u32), NativeEmitError> {
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
        } else if field.ty.name == "Array" {
            let array = self.array_layout_for(&field.ty)?;
            (array.size, array.alignment)
        } else {
            self.type_layout(ty)?
        };
        let layout = FieldLayout {
            name: field.name.clone(),
            offset: 0,
            ty,
            owned: matches!(field.role, StructFieldRole::Erg),
            indirect,
        };
        Ok((layout, size, field_alignment))
    }

    pub(in crate::codegen) fn native_type(
        &self,
        name: &str,
        visiting: &mut Vec<String>,
    ) -> Result<NativeType, NativeEmitError> {
        if let Some(primitive) = primitive_type(name) {
            return Ok(NativeType::from_primitive(primitive));
        }
        if let Some(native_type) = builtin_native_type(name)? {
            return Ok(native_type);
        }
        if let Some(enum_id) = self.enum_id_for(name) {
            self.ensure_enum_layout(name, enum_id, visiting)?;
            return Ok(NativeType::Enum(enum_id));
        }
        if let Some(pack_id) = self.pack_ids.get(name).copied() {
            return Ok(NativeType::Pack(pack_id));
        }
        let id = self
            .id_for(name)
            .ok_or_else(|| NativeEmitError(format!("unknown layout type `{name}`")))?;
        self.struct_native_type(name, id, visiting)
    }

    fn struct_native_type(
        &self,
        name: &str,
        id: usize,
        visiting: &mut Vec<String>,
    ) -> Result<NativeType, NativeEmitError> {
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

    fn ensure_enum_layout(
        &self,
        name: &str,
        enum_id: usize,
        visiting: &mut Vec<String>,
    ) -> Result<(), NativeEmitError> {
        let definition = self
            .enum_definitions
            .get(enum_id)
            .ok_or_else(|| NativeEmitError(format!("missing enum layout type `{name}`")))?;
        self.enum_layout_for(definition, visiting).map(|_| ())
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
            NativeType::Integer { width, .. } => integer_layout(width)?,
            NativeType::Float { width } => float_layout(width),
            NativeType::Void => (0, 1),
            NativeType::String | NativeType::Buffer => (self.pointer_size, self.pointer_size),
            NativeType::FatPointer => (self.pointer_size * 2, self.pointer_size),
            NativeType::Struct(id) => self.struct_type_layout(id)?,
            NativeType::Enum(id) => self.enum_type_layout(id)?,
            NativeType::Pack(id) => self.pack_type_layout(id)?,
            NativeType::Array(id) => self
                .array_layout_value(id)
                .map(|layout| (layout.size, layout.alignment))
                .ok_or_else(|| NativeEmitError("missing array layout".to_owned()))?,
            NativeType::Arena(capacity) => (capacity + self.pointer_size, self.pointer_size),
        })
    }

    fn struct_type_layout(&self, id: usize) -> Result<(u32, u32), NativeEmitError> {
        let definition = self
            .definitions
            .get(id)
            .ok_or_else(|| NativeEmitError(format!("missing nested struct layout `{id}")))?;
        let layout = self.layout_for(definition, &mut Vec::new())?;
        Ok((layout.size, layout.alignment))
    }

    fn enum_type_layout(&self, id: usize) -> Result<(u32, u32), NativeEmitError> {
        let layout = if let Some(layout) = self.enum_layout(id) {
            layout.clone()
        } else {
            let definition = self
                .enum_definitions
                .get(id)
                .ok_or_else(|| NativeEmitError(format!("missing enum layout `{id}")))?;
            self.enum_layout_for(definition, &mut Vec::new())?
        };
        Ok((layout.size, layout.alignment))
    }

    fn pack_type_layout(&self, id: usize) -> Result<(u32, u32), NativeEmitError> {
        if let Some(definition) = self.pack_definitions.get(id)
            && let (Some(size), Some(alignment)) = (
                definition.storage.byte_capacity().and_then(|size| u32::try_from(size).ok()),
                definition
                    .storage
                    .alignment_bytes()
                    .and_then(|alignment| u32::try_from(alignment).ok()),
            )
        {
            return Ok((size, alignment));
        }
        let pack =
            self.pack(id).ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
        self.type_layout(pack.storage)
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
            NativeType::Integer { width, .. } => integer_storage_bytes(width).ok_or_else(|| {
                NativeEmitError(format!("invalid native integer width `{width}`"))
            })?,
            NativeType::Float { width } => u32::from(width / 8),
            NativeType::Int => 4,
            NativeType::String | NativeType::Buffer | NativeType::Arena(_) => self.pointer_size,
            NativeType::FatPointer => self.pointer_size,
            NativeType::Void => 1,
            NativeType::Pack(id) => self
                .pack_definitions
                .get(id)
                .and_then(|definition| definition.storage.alignment_bytes())
                .and_then(|alignment| u32::try_from(alignment).ok())
                .map_or_else(
                    || {
                        self.pack(id)
                            .ok_or_else(|| {
                                NativeEmitError(format!("missing packed alignment `{id}`"))
                            })
                            .and_then(|pack| self.alignment(pack.storage))
                    },
                    Ok,
                )?,
            NativeType::Array(id) => self
                .array_layout_value(id)
                .map(|layout| layout.alignment)
                .ok_or_else(|| NativeEmitError("missing array alignment".to_owned()))?,
        })
    }
}

fn builtin_native_type(name: &str) -> Result<Option<NativeType>, NativeEmitError> {
    let Some(ty) = lookup_builtin_type(name) else { return Ok(None) };
    let native_type = match ty {
        BuiltinType::Int | BuiltinType::Bool => NativeType::Int,
        BuiltinType::String => NativeType::String,
        BuiltinType::Buffer => NativeType::Buffer,
        BuiltinType::Array | BuiltinType::Map => {
            return Err(NativeEmitError(format!("unsupported layout type `{name}`")));
        }
    };
    Ok(Some(native_type))
}

fn integer_layout(width: u8) -> Result<(u32, u32), NativeEmitError> {
    let bytes = integer_storage_bytes(width)
        .ok_or_else(|| NativeEmitError(format!("invalid native integer width `{width}`")))?;
    Ok((bytes, bytes))
}

fn float_layout(width: u8) -> (u32, u32) {
    let bytes = u32::from(width / 8);
    (bytes, bytes)
}

use std::collections::HashMap;

use cranelift_codegen::ir::{StackSlotData, StackSlotKind, Type};

use crate::ast::{
    BuiltinType, EnumDef, LayoutEndianness, PackDecl, Program, StructDef, StructFieldRole,
    TopLevelDecl, TypeName, lookup_builtin_type, primitive_type,
};
use crate::semantic::GenericInstance;

use super::enum_layout::EnumLayout;
use super::generic_definitions::{canonical_type_name, specialized_enums, specialized_structs};
use super::native::NativeEmitError;
use super::types::NativeType;

#[derive(Clone, Debug)]
pub(super) struct FieldLayout {
    pub(super) name: String,
    pub(super) offset: u32,
    pub(super) ty: NativeType,
    pub(super) owned: bool,
}

#[derive(Clone, Debug)]
pub(super) struct StructLayout {
    pub(super) size: u32,
    pub(super) alignment: u32,
    pub(super) fields: Vec<FieldLayout>,
}

#[derive(Clone, Debug)]
pub(super) struct PackFieldLayout {
    pub(super) name: String,
    pub(super) role: StructFieldRole,
    pub(super) ty: NativeType,
    pub(super) offset: u16,
    pub(super) width: u8,
}

#[derive(Clone, Debug)]
pub(super) struct PackLayout {
    pub(super) storage: NativeType,
    pub(super) endianness: LayoutEndianness,
    pub(super) fields: Vec<PackFieldLayout>,
}

pub struct LayoutRegistry {
    pub(super) pointer_type: Type,
    pub(super) pointer_size: u32,
    definitions: Vec<StructDef>,
    layouts: Vec<StructLayout>,
    ids: HashMap<String, usize>,
    pub(super) enum_definitions: Vec<EnumDef>,
    pub(super) enum_layouts: Vec<EnumLayout>,
    pub(super) enum_ids: HashMap<String, usize>,
    pack_definitions: Vec<PackDecl>,
    pub(super) pack_layouts: Vec<PackLayout>,
    pack_ids: HashMap<String, usize>,
}

impl LayoutRegistry {
    // Retained for layout unit tests and non-generic callers.
    #[allow(dead_code)]
    pub(super) fn from_program(
        program: &Program,
        pointer_type: Type,
    ) -> Result<Self, NativeEmitError> {
        Self::from_program_with_instances(program, pointer_type, &[])
    }

    pub(super) fn from_program_with_instances(
        program: &Program,
        pointer_type: Type,
        instances: &[GenericInstance],
    ) -> Result<Self, NativeEmitError> {
        let (mut definitions, mut enum_definitions) = base_definitions(program);
        definitions.extend(specialized_structs(program, instances)?);
        let ids = definitions
            .iter()
            .enumerate()
            .map(|(id, definition)| (definition.name.clone(), id))
            .collect::<HashMap<_, _>>();
        enum_definitions.extend(specialized_enums(program, instances)?);
        let enum_ids = enum_definitions
            .iter()
            .enumerate()
            .map(|(id, definition)| (definition.name.clone(), id))
            .collect::<HashMap<_, _>>();
        let capacity = definitions.len();
        let enum_capacity = enum_definitions.len();
        let pack_definitions = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Pack(pack) => Some(pack.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let pack_ids = pack_definitions
            .iter()
            .enumerate()
            .map(|(id, pack)| (pack.name.clone(), id))
            .collect::<HashMap<_, _>>();
        let mut registry = Self {
            pointer_type,
            pointer_size: pointer_type.bytes(),
            definitions,
            layouts: Vec::with_capacity(capacity),
            ids,
            enum_definitions,
            enum_layouts: Vec::with_capacity(enum_capacity),
            enum_ids,
            pack_definitions,
            pack_layouts: Vec::new(),
            pack_ids,
        };
        for definition in registry.definitions.clone() {
            registry.layouts.push(registry.layout_for(&definition, &mut Vec::new())?);
        }
        for definition in registry.enum_definitions.clone() {
            registry.enum_layouts.push(registry.enum_layout_for(&definition, &mut Vec::new())?);
        }
        registry.pack_layouts = registry
            .pack_definitions
            .iter()
            .map(|pack| registry.pack_layout_for(pack))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(registry)
    }

    pub(super) fn id_for(&self, name: &str) -> Option<usize> {
        self.ids.get(name).copied()
    }

    pub(super) fn get(&self, id: usize) -> Option<&StructLayout> {
        self.layouts.get(id)
    }

    pub(super) fn pack(&self, id: usize) -> Option<&PackLayout> {
        self.pack_layouts.get(id)
    }

    pub(super) fn pack_id(&self, name: &str) -> Option<usize> {
        self.pack_ids.get(name).copied()
    }

    pub(super) fn ir_type(&self, ty: NativeType) -> Type {
        match ty {
            NativeType::Pack(id) => self
                .pack(id)
                .map(|pack| self.ir_type(pack.storage))
                .unwrap_or(cranelift_codegen::ir::types::I32),
            NativeType::Arena(_) => self.pointer_type,
            _ => ty.ir_type(self.pointer_type),
        }
    }

    pub(super) fn type_for_name(&self, name: &str) -> Option<NativeType> {
        self.id_for(name)
            .map(NativeType::Struct)
            .or_else(|| self.enum_id_for(name).map(NativeType::Enum))
            .or_else(|| self.pack_ids.get(name).copied().map(NativeType::Pack))
    }

    pub(super) fn type_for_type_name(&self, type_name: &TypeName) -> Option<NativeType> {
        let canonical = canonical_type_name(type_name);
        self.type_for_name(&canonical).or_else(|| self.type_for_name(&type_name.name))
    }

    pub(super) fn stack_slot(&self, layout: &StructLayout) -> StackSlotData {
        StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            layout.size,
            layout.alignment.trailing_zeros() as u8,
        )
    }

    fn layout_for(
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
            let ty = self.native_type(&field.ty.name, visiting)?;
            let (size, field_alignment) = self.type_layout(ty)?;
            offset = align_up(offset, field_alignment);
            fields.push(FieldLayout {
                name: field.name.clone(),
                offset,
                ty,
                owned: matches!(field.role, StructFieldRole::Erg),
            });
            offset += size;
            alignment = alignment.max(field_alignment);
        }
        visiting.pop();
        Ok(StructLayout { size: align_up(offset, alignment), alignment, fields })
    }

    pub(super) fn native_type(
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
            .ok_or_else(|| NativeEmitError(format!("missing layout type `{name}`")))?;
        self.layout_for(definition, visiting)?;
        Ok(NativeType::Struct(id))
    }

    pub(super) fn type_layout(&self, ty: NativeType) -> Result<(u32, u32), NativeEmitError> {
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
                    NativeEmitError(format!("missing nested struct layout `{id}`"))
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
                        .ok_or_else(|| NativeEmitError(format!("missing enum layout `{id}`")))?;
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

    pub(super) fn alignment(&self, ty: NativeType) -> Result<u32, NativeEmitError> {
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

impl LayoutRegistry {
    fn pack_layout_for(&self, pack: &PackDecl) -> Result<PackLayout, NativeEmitError> {
        let storage = self.native_type(&pack.storage.name, &mut Vec::new())?;
        let fields = pack
            .fields
            .iter()
            .map(|field| {
                let width = crate::ast::primitive_type(&field.ty.name)
                    .and_then(|primitive| match primitive {
                        crate::ast::PrimitiveType::Integer { width, .. } => Some(width),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        NativeEmitError(format!("invalid packed field `{}`", field.name))
                    })?;
                let role = match field.role {
                    crate::ast::Role::Erg => StructFieldRole::Erg,
                    _ => StructFieldRole::Value,
                };
                Ok(PackFieldLayout {
                    name: field.name.clone(),
                    role,
                    ty: self.native_type(&field.ty.name, &mut Vec::new())?,
                    offset: field.offset,
                    width,
                })
            })
            .collect::<Result<Vec<_>, NativeEmitError>>()?;
        Ok(PackLayout { storage, endianness: pack.endianness, fields })
    }
}

fn integer_storage_bytes(width: u8) -> u32 {
    match width {
        1..=8 => 1,
        9..=16 => 2,
        17..=32 => 4,
        33..=64 => 8,
        65..=128 => 16,
        _ => 0,
    }
}

fn base_definitions(program: &Program) -> (Vec<StructDef>, Vec<EnumDef>) {
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Struct(definition) if definition.generic_parameters.is_empty() => {
                structs.push(definition.clone());
            }
            TopLevelDecl::Enum(definition) if definition.generic_parameters.is_empty() => {
                enums.push(definition.clone());
            }
            _ => {}
        }
    }
    (structs, enums)
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
    fn calculates_natural_offsets_and_trailing_padding() {
        let (tokens, errors) = scan("struct Point { x: Int, y: Int, }");
        assert!(errors.is_empty());
        let program = parse(tokens).expect("struct should parse");
        let layouts =
            LayoutRegistry::from_program(&program, types::I64).expect("layout should pass");
        let layout =
            layouts.get(layouts.id_for("Point").expect("Point layout should exist")).unwrap();

        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.size, 8);
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[1].offset, 4);
    }

    #[test]
    fn calculates_nested_struct_offsets() {
        let (tokens, errors) =
            scan("struct Inner { x: Int, y: Int, } struct Outer { inner: Inner, tag: Int, }");
        assert!(errors.is_empty());
        let program = parse(tokens).expect("nested structs should parse");
        let layouts =
            LayoutRegistry::from_program(&program, types::I64).expect("layout should pass");
        let outer =
            layouts.get(layouts.id_for("Outer").expect("Outer layout should exist")).unwrap();

        assert_eq!(outer.alignment, 4);
        assert_eq!(outer.size, 12);
        assert_eq!(outer.fields[0].offset, 0);
        assert_eq!(outer.fields[1].offset, 8);
    }
}

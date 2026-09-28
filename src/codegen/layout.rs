use std::collections::HashMap;

use cranelift_codegen::ir::{StackSlotData, StackSlotKind, Type};

use crate::ast::{
    EnumDef, LayoutEndianness, PackDecl, Program, StructDef, StructFieldRole, TopLevelDecl,
    TypeName,
};
use crate::semantic::GenericInstance;

use super::enum_layout::EnumLayout;
use super::generic_definitions::{canonical_type_name, specialized_enums, specialized_structs};
use super::native::NativeEmitError;
use super::types::NativeType;

#[path = "layout_structs.rs"]
mod layout_structs;

#[derive(Clone, Debug)]
pub(super) struct FieldLayout {
    pub(super) name: String,
    pub(super) offset: u32,
    pub(super) ty: NativeType,
    pub(super) owned: bool,
    pub(super) indirect: bool,
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

    pub(super) fn ir_type(&self, ty: NativeType) -> Result<Type, NativeEmitError> {
        match ty {
            NativeType::Pack(id) => self
                .pack(id)
                .ok_or_else(|| NativeEmitError(format!("missing packed layout `{id}`")))
                .and_then(|pack| self.ir_type(pack.storage)),
            NativeType::Arena(_) => Ok(self.pointer_type),
            _ => Ok(ty.ir_type(self.pointer_type)),
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
        self.type_for_name(&canonical)
            .or_else(|| self.type_for_name(&type_name.name))
            .or_else(|| NativeType::from_name(&type_name.name))
    }

    pub(super) fn stack_slot(&self, layout: &StructLayout) -> StackSlotData {
        StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            layout.size,
            layout.alignment.trailing_zeros() as u8,
        )
    }

    pub(super) fn uses_return_slot(&self, ty: NativeType) -> bool {
        ty.uses_sret() || self.is_borrowed_view_option(ty)
    }

    pub(super) fn returns_borrowed_view(&self, ty: NativeType) -> bool {
        self.is_borrowed_view_option(ty)
    }

    pub(super) fn return_slot_size(&self, ty: NativeType) -> Option<u32> {
        if let NativeType::Enum(id) = ty
            && self.is_borrowed_view_option(ty)
        {
            return self.niche_payload_type(id).and_then(|payload| self.type_size(payload));
        }
        self.type_size(ty)
    }

    fn is_borrowed_view_option(&self, ty: NativeType) -> bool {
        let NativeType::Enum(id) = ty else { return false };
        let Some(layout) = self.enum_layout(id) else { return false };
        layout.niche_pointer
            && self
                .niche_payload_type(id)
                .is_some_and(|payload| self.is_borrowed_descriptor(payload))
    }

    fn niche_payload_type(&self, id: usize) -> Option<NativeType> {
        let definition = self.enum_definitions.get(id)?;
        let variant = definition.variants.iter().find(|variant| variant.name == "Some")?;
        let crate::ast::EnumPayload::Tuple(types) = &variant.payload else { return None };
        let payload = types.first()?;
        self.type_for_type_name(payload)
    }

    fn is_borrowed_descriptor(&self, ty: NativeType) -> bool {
        let NativeType::Struct(id) = ty else { return false };
        self.definitions.get(id).is_some_and(|definition| {
            definition
                .fields
                .iter()
                .any(|field| matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins))
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

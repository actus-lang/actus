use std::collections::HashMap;

use cranelift_codegen::ir::{StackSlotData, StackSlotKind, Type};

use crate::ast::{
    EnumDef, LayoutEndianness, PackDecl, Program, StructDef, StructFieldRole, TopLevelDecl,
    TypeName, builtin_enum_definitions,
};
use crate::semantic::GenericInstance;

use super::enum_layout::EnumLayout;
use super::generic::{canonical_type_name, specialized_enums, specialized_structs};
use super::native::NativeEmitError;
use super::types::NativeType;

mod arrays;
mod packs;
mod structs;

pub(super) use arrays::array_definitions;

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
    pub(super) indexed_count: Option<u32>,
}

#[derive(Clone, Debug)]
pub(super) struct PackLayout {
    pub(super) storage: NativeType,
    pub(super) endianness: LayoutEndianness,
    pub(super) fields: Vec<PackFieldLayout>,
}

#[derive(Clone, Debug)]
pub(super) struct ArrayLayout {
    pub(super) element: NativeType,
    pub(super) capacity: u32,
    pub(super) size: u32,
    pub(super) alignment: u32,
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
    pub(super) pack_definitions: Vec<PackDecl>,
    pub(super) pack_layouts: Vec<PackLayout>,
    pack_ids: HashMap<String, usize>,
    array_definitions: Vec<TypeName>,
    array_layouts: Vec<ArrayLayout>,
    array_ids: HashMap<String, usize>,
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
        let (definitions, enum_definitions) = specialized_definitions(program, instances)?;
        let pack_definitions = pack_definitions(program);
        let array_definitions = array_definitions(program, &definitions, &enum_definitions);
        let mut registry = Self::new(
            pointer_type,
            definitions,
            enum_definitions,
            pack_definitions,
            array_definitions,
        );
        registry.populate_layouts()?;
        Ok(registry)
    }

    fn new(
        pointer_type: Type,
        definitions: Vec<StructDef>,
        enum_definitions: Vec<EnumDef>,
        pack_definitions: Vec<PackDecl>,
        array_definitions: Vec<TypeName>,
    ) -> Self {
        let ids = named_ids(&definitions, |definition| &definition.name);
        let enum_ids = named_ids(&enum_definitions, |definition| &definition.name);
        let pack_ids = named_ids(&pack_definitions, |definition| &definition.name);
        Self {
            pointer_type,
            pointer_size: pointer_type.bytes(),
            layouts: Vec::with_capacity(definitions.len()),
            enum_layouts: Vec::with_capacity(enum_definitions.len()),
            definitions,
            ids,
            enum_definitions,
            enum_ids,
            pack_definitions,
            pack_layouts: Vec::new(),
            pack_ids,
            array_ids: array_ids(&array_definitions),
            array_definitions,
            array_layouts: Vec::new(),
        }
    }

    fn populate_layouts(&mut self) -> Result<(), NativeEmitError> {
        for definition in self.definitions.clone() {
            self.layouts.push(self.layout_for(&definition, &mut Vec::new())?);
        }
        for definition in self.enum_definitions.clone() {
            self.enum_layouts.push(self.enum_layout_for(&definition, &mut Vec::new())?);
        }
        self.pack_layouts = self
            .pack_definitions
            .iter()
            .map(|pack| self.pack_layout_for(pack))
            .collect::<Result<Vec<_>, _>>()?;
        self.array_layouts.clear();
        for array in self.array_definitions.clone() {
            self.array_layouts.push(self.array_layout_for(&array)?);
        }
        Ok(())
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

    pub(super) fn array(&self, id: usize) -> Option<&ArrayLayout> {
        self.array_layouts.get(id)
    }

    pub(super) fn array_id(&self, canonical: &str) -> Option<usize> {
        self.array_ids.get(canonical).copied().or_else(|| {
            let normalized = canonical
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            self.array_ids.get(&normalized).copied()
        })
    }

    pub(super) fn ir_type(&self, ty: NativeType) -> Result<Type, NativeEmitError> {
        match ty {
            NativeType::Pack(id) => self
                .pack(id)
                .ok_or_else(|| NativeEmitError(format!("missing packed layout `{id}`")))
                .and_then(|pack| self.ir_type(pack.storage)),
            NativeType::Array(_) => Ok(self.pointer_type),
            NativeType::Arena(_) => Ok(self.pointer_type),
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
        self.type_for_name(&canonical)
            .or_else(|| self.array_ids.get(&canonical).copied().map(NativeType::Array))
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
        ty.uses_sret() || self.is_inline_pack(ty) || self.is_borrowed_view_option(ty)
    }

    pub(super) fn is_inline_pack(&self, ty: NativeType) -> bool {
        let NativeType::Pack(id) = ty else { return false };
        self.pack(id).is_some_and(|pack| matches!(pack.storage, NativeType::Array(_)))
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

pub(super) fn integer_storage_bytes(width: u8) -> Option<u32> {
    match width {
        1..=8 => Some(1),
        9..=16 => Some(2),
        17..=32 => Some(4),
        33..=64 => Some(8),
        65..=128 => Some(16),
        _ => None,
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
    enums.extend(
        builtin_enum_definitions()
            .into_iter()
            .filter(|definition| definition.generic_parameters.is_empty()),
    );
    (structs, enums)
}

fn specialized_definitions(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<(Vec<StructDef>, Vec<EnumDef>), NativeEmitError> {
    let (mut definitions, mut enum_definitions) = base_definitions(program);
    definitions.extend(specialized_structs(program, instances)?);
    enum_definitions.extend(specialized_enums(program, instances)?);
    Ok((definitions, enum_definitions))
}

fn pack_definitions(program: &Program) -> Vec<PackDecl> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Pack(pack) => Some(pack.clone()),
            _ => None,
        })
        .collect()
}

fn array_ids(definitions: &[TypeName]) -> HashMap<String, usize> {
    definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| (canonical_type_name(definition), index))
        .collect()
}

fn named_ids<T>(definitions: &[T], name: impl Fn(&T) -> &String) -> HashMap<String, usize> {
    definitions.iter().enumerate().map(|(id, definition)| (name(definition).clone(), id)).collect()
}

fn align_up(offset: u32, alignment: u32) -> u32 {
    offset.div_ceil(alignment) * alignment
}

use std::collections::BTreeMap;

use crate::ast::{
    BuiltinType, EnumDef, EnumPayload, PackDecl, Program, StructDef, StructFieldRole, TopLevelDecl,
    TypeName, primitive_type,
};

use super::super::BackendFieldLayout;
use super::super::symbols::{SymbolIdentity, SymbolKind};
use super::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupProvider, BackendCleanupResolvedDescriptor,
    BackendCleanupResult, BackendContractError, BackendLayout, BackendLayoutProvider,
};

/// Program-backed layouts used by the backend-neutral signature contract.
///
/// The provider owns no target backend types. It derives nominal struct layout
/// from the Actus declarations and uses the host pointer width only for
/// reference roles and runtime handles, matching the existing neutral ABI.
pub(super) struct ProgramLayouts<'program> {
    structs: BTreeMap<&'program str, &'program StructDef>,
    enums: BTreeMap<&'program str, &'program EnumDef>,
    packs: BTreeMap<&'program str, &'program PackDecl>,
    drop_symbols: BTreeMap<String, SymbolIdentity>,
    pointer_bytes: u32,
}

include!("layout/options.rs");

impl<'program> ProgramLayouts<'program> {
    pub(super) fn new(program: &'program Program, namespace: &str) -> Self {
        let structs = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Struct(definition) if definition.generic_parameters.is_empty() => {
                    Some((definition.name.as_str(), definition))
                }
                _ => None,
            })
            .collect();
        let packs = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Pack(definition) => Some((definition.name.as_str(), definition)),
                _ => None,
            })
            .collect();
        let enums = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Enum(definition) if definition.generic_parameters.is_empty() => {
                    Some((definition.name.as_str(), definition))
                }
                _ => None,
            })
            .collect();
        let drop_symbols = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Perform(perform) if perform.role_name == "Drop" => {
                    let target = perform.target.canonical_key();
                    let symbol = SymbolIdentity::specialized(
                        namespace,
                        SymbolKind::Performance,
                        "drop",
                        &["Drop", &target],
                    )
                    .ok()?;
                    Some((target, symbol))
                }
                _ => None,
            })
            .collect();
        Self { structs, enums, packs, drop_symbols, pointer_bytes: usize::BITS / 8 }
    }

    fn type_layout(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Option<BackendLayout> {
        if is_niche_pointer_option(type_name) {
            return Some(self.pointer_layout());
        }
        if matches!(type_name.name.as_str(), "Arena" | "Buffer" | "Region" | "String") {
            return Some(self.pointer_layout());
        }
        if type_name.name == "Int" {
            return Some(Self::scalar_layout(4));
        }
        if type_name.name == "Bool" {
            return Some(Self::scalar_layout(1));
        }
        if let Some(primitive) = primitive_type(&type_name.name) {
            return match primitive {
                crate::ast::PrimitiveType::Integer { width, .. } => Self::integer_layout(width),
                crate::ast::PrimitiveType::Float { width } => {
                    (width % 8 == 0).then(|| Self::scalar_layout(u32::from(width / 8)))
                }
                crate::ast::PrimitiveType::Void => Some(Self::scalar_layout(0)),
            };
        }
        if type_name.name == "Array" {
            return self.array_layout(type_name, visiting);
        }
        if self.enums.contains_key(type_name.name.as_str()) {
            return self
                .named_enum_layout(type_name.name.as_str(), visiting)
                .map(|layout| layout.layout);
        }
        if let Some(pack) = self.packs.get(type_name.name.as_str()) {
            return self.type_layout(pack.storage.type_name(), visiting);
        }
        self.struct_layout(&type_name.name, visiting)
    }

    fn struct_layout(&self, name: &str, visiting: &mut Vec<String>) -> Option<BackendLayout> {
        if visiting.iter().any(|candidate| candidate == name) {
            return None;
        }
        let definition = self.structs.get(name)?;
        visiting.push(name.to_owned());
        let mut size = 0u32;
        let mut alignment = 1u32;
        for field in &definition.fields {
            let field_layout = if matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins)
            {
                self.pointer_layout()
            } else {
                self.type_layout(&field.ty, visiting)?
            };
            size = Self::align_up(size, field_layout.alignment)?;
            size = size.checked_add(field_layout.size)?;
            alignment = alignment.max(field_layout.alignment);
        }
        visiting.pop();
        Some(BackendLayout { size: Self::align_up(size, alignment)?, alignment })
    }

    fn array_layout(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Option<BackendLayout> {
        let [element, capacity] = type_name.arguments.as_slice() else {
            return None;
        };
        let capacity = capacity.name.parse::<u32>().ok()?;
        let element = self.type_layout(element, visiting)?;
        let size = element.size.checked_mul(capacity)?;
        Some(BackendLayout {
            size: Self::align_up(size, element.alignment)?,
            alignment: element.alignment,
        })
    }

    fn named_enum_layout(
        &self,
        name: &str,
        visiting: &mut Vec<String>,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        if visiting.iter().any(|candidate| candidate == name) {
            return None;
        }
        let definition = self.enums.get(name)?;
        visiting.push(name.to_owned());
        let mut payload: Option<BackendLayout> = None;
        for variant in &definition.variants {
            let variant_payload = match &variant.payload {
                EnumPayload::Unit => None,
                EnumPayload::Tuple(types) => self.sequence_layout(types, visiting),
                EnumPayload::Struct(fields) => self.sequence_layout(
                    &fields.iter().map(|field| field.ty.clone()).collect::<Vec<_>>(),
                    visiting,
                ),
            };
            if let Some(candidate) = variant_payload {
                payload = Some(match payload {
                    Some(current) if current.size >= candidate.size => current,
                    _ => candidate,
                });
            }
        }
        visiting.pop();
        let payload_offset = match payload {
            Some(layout) => Self::align_up(4, layout.alignment)?,
            None => 4,
        };
        let alignment = payload.map_or(4, |layout| 4u32.max(layout.alignment));
        let size = Self::align_up(
            payload_offset.checked_add(payload.map_or(0, |layout| layout.size))?,
            alignment,
        )?;
        Some(super::super::abi::BackendEnumLayout {
            layout: BackendLayout { size, alignment },
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset,
            payload,
            niche_pointer: false,
        })
    }

    fn sequence_layout(
        &self,
        types: &[TypeName],
        visiting: &mut Vec<String>,
    ) -> Option<BackendLayout> {
        let mut size = 0u32;
        let mut alignment = 1u32;
        for type_name in types {
            let field = self.type_layout(type_name, visiting)?;
            size = Self::align_up(size, field.alignment)?;
            size = size.checked_add(field.size)?;
            alignment = alignment.max(field.alignment);
        }
        Some(BackendLayout { size: Self::align_up(size, alignment)?, alignment })
    }

    fn pointer_layout(&self) -> BackendLayout {
        Self::scalar_layout(self.pointer_bytes)
    }

    fn scalar_layout(size: u32) -> BackendLayout {
        BackendLayout { size, alignment: size.max(1) }
    }

    fn integer_layout(width: u8) -> Option<BackendLayout> {
        let size = match width {
            1..=8 => 1,
            9..=16 => 2,
            17..=32 => 4,
            33..=64 => 8,
            65..=128 => 16,
            _ => return None,
        };
        Some(Self::scalar_layout(size))
    }

    fn align_up(value: u32, alignment: u32) -> Option<u32> {
        if alignment == 0 || !alignment.is_power_of_two() {
            return None;
        }
        value.checked_add(alignment - 1).map(|rounded| rounded & !(alignment - 1))
    }

    fn field_storage_layout(
        &self,
        role: &StructFieldRole,
        type_name: &TypeName,
    ) -> Option<BackendLayout> {
        if matches!(role, StructFieldRole::Abs | StructFieldRole::Ins) {
            Some(self.pointer_layout())
        } else {
            self.type_layout(type_name, &mut Vec::new())
        }
    }

    fn cleanup_field(
        &self,
        root_type: &TypeName,
        path: &str,
    ) -> Option<(TypeName, BackendCleanupField)> {
        let mut current_name = root_type.name.clone();
        let mut offset = 0u32;
        let mut segments = path.split('.').filter(|segment| !segment.is_empty()).peekable();
        while let Some(segment) = segments.next() {
            let definition = self.structs.get(current_name.as_str())?;
            let mut field_offset = 0u32;
            let mut selected = None;
            for field in &definition.fields {
                let layout = self.field_storage_layout(&field.role, &field.ty)?;
                field_offset = Self::align_up(field_offset, layout.alignment)?;
                if field.name == segment {
                    selected = Some((field, layout));
                    break;
                }
                field_offset = field_offset.checked_add(layout.size)?;
            }
            let (field, layout) = selected?;
            let absolute_offset = offset.checked_add(field_offset)?;
            if segments.peek().is_none() {
                return Some((
                    field.ty.clone(),
                    BackendCleanupField {
                        offset: absolute_offset,
                        size: layout.size,
                        alignment: layout.alignment,
                    },
                ));
            }
            if matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins) {
                return None;
            }
            current_name = field.ty.name.clone();
            offset = absolute_offset;
        }
        None
    }

    fn cleanup_descriptor(
        &self,
        bridge: BackendCleanupBridge,
        layout: BackendLayout,
        field: Option<BackendCleanupField>,
    ) -> BackendCleanupResolvedDescriptor {
        BackendCleanupResolvedDescriptor {
            descriptor: BackendCleanupBridgeDescriptor {
                bridge,
                argument: if field.is_some() {
                    BackendCleanupArgument::FieldAddress
                } else if bridge == BackendCleanupBridge::BufferDrop {
                    BackendCleanupArgument::Value
                } else {
                    BackendCleanupArgument::Address
                },
                result: if bridge == BackendCleanupBridge::RegionDrop {
                    BackendCleanupResult::Status
                } else {
                    BackendCleanupResult::Ignored
                },
            },
            layout,
            field,
            performance_symbol: None,
        }
    }
}

impl BackendLayoutProvider for ProgramLayouts<'_> {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
        self.type_layout(type_name, &mut Vec::new())
    }

    fn enum_layout_for(
        &self,
        type_name: &TypeName,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        if let Some(layout) = self.niche_option_layout(type_name) {
            return Some(layout);
        }
        if type_name.name == "Result" && type_name.arguments.len() == 2 {
            let payload = type_name
                .arguments
                .iter()
                .filter(|argument| argument.name != "Void")
                .filter_map(|argument| self.type_layout(argument, &mut Vec::new()))
                .max_by_key(|layout| (layout.size, layout.alignment));
            let payload_offset = match payload.as_ref() {
                Some(layout) => Self::align_up(4, layout.alignment)?,
                None => 4,
            };
            let alignment = payload.as_ref().map_or(4, |layout| 4u32.max(layout.alignment));
            return Some(super::super::abi::BackendEnumLayout {
                layout: BackendLayout {
                    size: Self::align_up(
                        payload_offset
                            .checked_add(payload.as_ref().map_or(0, |layout| layout.size))?,
                        alignment,
                    )?,
                    alignment,
                },
                discriminant_offset: 0,
                discriminant_size: 4,
                payload_offset,
                payload,
                niche_pointer: false,
            });
        }
        self.named_enum_layout(type_name.name.as_str(), &mut Vec::new())
    }

    fn enum_variant(&self, type_name: &TypeName, variant: &str) -> Option<(u32, bool)> {
        if let Some(variant) = niche_option_variant(type_name, variant) {
            return Some(variant);
        }
        let definition = self.enums.get(type_name.name.as_str())?;
        let (index, definition) = definition
            .variants
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.name == variant)?;
        Some((index as u32, matches!(definition.payload, EnumPayload::Unit)))
    }

    fn enum_variant_for_layout(
        &self,
        layout: super::super::abi::BackendEnumLayout,
        variant: &str,
    ) -> Option<(u32, bool)> {
        if layout.niche_pointer {
            return niche_pointer_variant(variant);
        }
        self.enums.values().find_map(|definition| {
            let type_name = TypeName {
                name: definition.name.clone(),
                arguments: Vec::new(),
                reference_role: None,
                span: definition.span,
            };
            (self.enum_layout_for(&type_name) == Some(layout))
                .then(|| self.enum_variant(&type_name, variant))
                .flatten()
        })
    }

    fn type_for_layout(&self, layout: BackendLayout) -> Option<TypeName> {
        self.packs
            .values()
            .find_map(|definition| {
                let type_name = TypeName {
                    name: definition.name.clone(),
                    arguments: Vec::new(),
                    reference_role: None,
                    span: definition.span,
                };
                (self.layout_for(&type_name) == Some(layout)).then_some(type_name)
            })
            .or_else(|| {
                self.structs.values().find_map(|definition| {
                    let type_name = TypeName {
                        name: definition.name.clone(),
                        arguments: Vec::new(),
                        reference_role: None,
                        span: definition.span,
                    };
                    (self.layout_for(&type_name) == Some(layout)).then_some(type_name)
                })
            })
    }

    fn packed_field_for(
        &self,
        type_name: &TypeName,
        field_name: &str,
    ) -> Option<super::super::BackendPackedFieldLayout> {
        let pack = self.packs.get(type_name.name.as_str())?;
        let field = pack.fields.iter().find(|field| field.name == field_name)?;
        let primitive = if field.ty.name == "Array" && field.ty.arguments.len() == 2 {
            &field.ty.arguments[0]
        } else {
            &field.ty
        };
        let width = match primitive_type(&primitive.name)? {
            crate::ast::PrimitiveType::Integer { width, .. } => width,
            _ => return None,
        };
        let storage = self.type_layout(pack.storage.type_name(), &mut Vec::new())?;
        Some(super::super::BackendPackedFieldLayout {
            storage,
            bit_offset: field.offset,
            width,
            big_endian: matches!(pack.endianness, crate::ast::LayoutEndianness::Big),
            ty: field.ty.clone(),
        })
    }

    fn field_for(&self, type_name: &TypeName, field_name: &str) -> Option<BackendFieldLayout> {
        if let Some(pack) = self.packs.get(type_name.name.as_str()) {
            if field_name != "storage" {
                return None;
            }
            let storage = pack.storage.type_name().clone();
            let layout = self.type_layout(&storage, &mut Vec::new())?;
            return Some(BackendFieldLayout { offset: 0, layout, ty: storage, indirect: false });
        }
        let definition = self.structs.get(type_name.name.as_str())?;
        let mut offset = 0u32;
        for field in &definition.fields {
            let layout = self.field_storage_layout(&field.role, &field.ty)?;
            offset = Self::align_up(offset, layout.alignment)?;
            if field.name == field_name {
                return Some(BackendFieldLayout {
                    offset,
                    layout,
                    ty: field.ty.clone(),
                    indirect: matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins),
                });
            }
            offset = offset.checked_add(layout.size)?;
        }
        None
    }
}

include!("layout/cleanup.rs");

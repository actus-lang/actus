use std::collections::BTreeMap;

use crate::ast::{
    BuiltinType, EnumDef, EnumPayload, PackDecl, Program, StructDef, StructFieldRole, TopLevelDecl,
    TypeName, primitive_type,
};

use super::super::symbols::{SymbolIdentity, SymbolKind};
use super::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupProvider, BackendCleanupResolvedDescriptor,
    BackendCleanupResult, BackendContractError, BackendLayout,
};

mod generic_struct;

/// Program-backed layouts used by the backend-neutral signature contract.
///
/// The provider owns no target backend types. It derives nominal struct layout
/// from the Actus declarations and uses the host pointer width only for
/// reference roles and runtime handles, matching the existing neutral ABI.
pub(crate) struct ProgramLayouts<'program> {
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
                TopLevelDecl::Struct(definition) => Some((definition.name.as_str(), definition)),
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
                TopLevelDecl::Enum(definition) => Some((definition.name.as_str(), definition)),
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
        if type_name.name == "Option" && type_name.arguments.len() == 1 {
            return self.enum_layout_for(type_name).map(|layout| layout.layout);
        }
        if self
            .structs
            .get(type_name.name.as_str())
            .is_some_and(|definition| !definition.generic_parameters.is_empty())
        {
            return self.generic_struct_layout(type_name, visiting);
        }
        if self.enums.contains_key(type_name.name.as_str()) {
            return self.named_enum_layout(type_name, visiting).map(|layout| layout.layout);
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
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        let key = type_name.canonical_key();
        if visiting.iter().any(|candidate| candidate == &key) {
            return None;
        }
        let definition = self.enums.get(type_name.name.as_str())?;
        let substitution = (!definition.generic_parameters.is_empty())
            .then(|| {
                crate::semantic::TypeSubstitution::for_type(
                    &definition.name,
                    &definition.generic_parameters,
                    &type_name.arguments,
                    definition.span,
                )
                .ok()
            })
            .flatten();
        visiting.push(key);
        let payload = self.largest_enum_payload(definition, substitution.as_ref(), visiting);
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

    fn largest_enum_payload(
        &self,
        definition: &EnumDef,
        substitution: Option<&crate::semantic::TypeSubstitution>,
        visiting: &mut Vec<String>,
    ) -> Option<BackendLayout> {
        definition
            .variants
            .iter()
            .filter_map(|variant| self.enum_variant_payload_layout(variant, substitution, visiting))
            .max_by_key(|layout| (layout.size, layout.alignment))
    }

    fn enum_variant_payload_layout(
        &self,
        variant: &crate::ast::EnumVariant,
        substitution: Option<&crate::semantic::TypeSubstitution>,
        visiting: &mut Vec<String>,
    ) -> Option<BackendLayout> {
        let types = match &variant.payload {
            EnumPayload::Unit => return None,
            EnumPayload::Tuple(types) => types
                .iter()
                .map(|ty| substitution.map_or_else(|| ty.clone(), |s| s.apply(ty)))
                .collect::<Vec<_>>(),
            EnumPayload::Struct(fields) => fields
                .iter()
                .map(|field| substitution.map_or_else(|| field.ty.clone(), |s| s.apply(&field.ty)))
                .collect::<Vec<_>>(),
        };
        self.sequence_layout(&types, visiting)
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
                } else if matches!(
                    bridge,
                    BackendCleanupBridge::BufferDrop | BackendCleanupBridge::AggregateDrop { .. }
                ) {
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

include!("layout/provider.rs");

include!("layout/cleanup.rs");

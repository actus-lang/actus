use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    BuiltinType, Program, StructDef, StructFieldRole, TopLevelDecl, TypeName, primitive_type,
};

use super::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupProvider, BackendCleanupResolvedDescriptor,
    BackendCleanupResult, BackendContractError, BackendLayout, BackendLayoutProvider,
    BackendTargetContext, BackendUnsupportedOperation,
};

/// Program-backed layouts used by the backend-neutral signature contract.
///
/// The provider owns no target backend types. It derives nominal struct layout
/// from the Actus declarations and uses the host pointer width only for
/// reference roles and runtime handles, matching the existing neutral ABI.
pub(super) struct ProgramLayouts<'program> {
    structs: BTreeMap<&'program str, &'program StructDef>,
    drop_types: BTreeSet<String>,
    pointer_bytes: u32,
}

impl<'program> ProgramLayouts<'program> {
    pub(super) fn new(program: &'program Program) -> Self {
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
        let drop_types = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Perform(perform) if perform.role_name == "Drop" => {
                    Some(perform.target.canonical_key())
                }
                _ => None,
            })
            .collect();
        Self { structs, drop_types, pointer_bytes: (usize::BITS / 8) as u32 }
    }

    fn type_layout(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Option<BackendLayout> {
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
        }
    }
}

impl BackendLayoutProvider for ProgramLayouts<'_> {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
        if matches!(type_name.name.as_str(), "Arena" | "Buffer" | "Region" | "String") {
            return Some(self.pointer_layout());
        }
        self.struct_layout(&type_name.name, &mut Vec::new())
    }

    fn enum_layout_for(
        &self,
        type_name: &TypeName,
    ) -> Option<super::super::abi::BackendEnumLayout> {
        if type_name.name != "Result" || type_name.arguments.len() != 2 {
            return None;
        }
        let payload = self.type_layout(type_name.arguments.first()?, &mut Vec::new())?;
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

impl BackendCleanupProvider for ProgramLayouts<'_> {
    fn resolve_binding(
        &self,
        type_name: Option<&TypeName>,
        builtin: Option<BuiltinType>,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        if builtin == Some(BuiltinType::Buffer)
            || type_name.is_some_and(|name| name.name == "Buffer")
        {
            return Ok(Some(self.cleanup_descriptor(
                BackendCleanupBridge::BufferDrop,
                self.pointer_layout(),
                None,
            )));
        }
        if type_name.is_some_and(|name| name.name == "Region") {
            return Ok(Some(self.cleanup_descriptor(
                BackendCleanupBridge::RegionDrop,
                self.pointer_layout(),
                None,
            )));
        }
        if let Some(type_name) = type_name
            && self.drop_types.contains(&type_name.canonical_key())
        {
            return Err(BackendContractError::Unsupported(BackendUnsupportedOperation::new(
                "cleanup-performance",
                type_name.span,
                BackendTargetContext {
                    backend: "neutral-contract".to_owned(),
                    target: "semantic-model".to_owned(),
                },
                format!(
                    "user-defined Drop performance for `{}` requires a backend performance bridge",
                    type_name.canonical_key()
                ),
            )));
        }
        Ok(None)
    }

    fn resolve_field(
        &self,
        root_type: &TypeName,
        path: &str,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        let Some(root_layout) = self.type_layout(root_type, &mut Vec::new()) else {
            return Ok(None);
        };
        let Some((field_type, field)) = self.cleanup_field(root_type, path) else {
            return Ok(None);
        };
        let bridge = match field_type.name.as_str() {
            "Buffer" => BackendCleanupBridge::BufferDrop,
            "Region" => BackendCleanupBridge::RegionDrop,
            _ => return Ok(None),
        };
        Ok(Some(self.cleanup_descriptor(bridge, root_layout, Some(field))))
    }
}

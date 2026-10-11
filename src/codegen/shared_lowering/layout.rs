use std::collections::BTreeMap;

use crate::ast::{Program, StructDef, StructFieldRole, TopLevelDecl, TypeName, primitive_type};

use super::{BackendLayout, BackendLayoutProvider};

/// Program-backed layouts used by the backend-neutral signature contract.
///
/// The provider owns no target backend types. It derives nominal struct layout
/// from the Actus declarations and uses the host pointer width only for
/// reference roles and runtime handles, matching the existing neutral ABI.
pub(super) struct ProgramLayouts<'program> {
    structs: BTreeMap<&'program str, &'program StructDef>,
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
        Self { structs, pointer_bytes: (usize::BITS / 8) as u32 }
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

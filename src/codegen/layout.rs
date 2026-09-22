use std::collections::HashMap;

use cranelift_codegen::ir::{StackSlotData, StackSlotKind, Type};

use crate::ast::{BuiltinType, Program, StructDef, TopLevelDecl, lookup_builtin_type};

use super::native::NativeEmitError;
use super::types::NativeType;

#[derive(Clone, Debug)]
pub(super) struct FieldLayout {
    pub(super) name: String,
    pub(super) offset: u32,
    pub(super) ty: NativeType,
}

#[derive(Clone, Debug)]
pub(super) struct StructLayout {
    pub(super) size: u32,
    pub(super) alignment: u32,
    pub(super) fields: Vec<FieldLayout>,
}

pub(super) struct LayoutRegistry {
    pub(super) pointer_type: Type,
    pub(super) pointer_size: u32,
    definitions: Vec<StructDef>,
    layouts: Vec<StructLayout>,
    ids: HashMap<String, usize>,
}

impl LayoutRegistry {
    pub(super) fn from_program(
        program: &Program,
        pointer_type: Type,
    ) -> Result<Self, NativeEmitError> {
        let definitions = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Struct(definition) => Some(definition),
                _ => None,
            })
            .collect::<Vec<_>>();
        let definitions = definitions.into_iter().cloned().collect::<Vec<_>>();
        let ids = definitions
            .iter()
            .enumerate()
            .map(|(id, definition)| (definition.name.clone(), id))
            .collect::<HashMap<_, _>>();
        let capacity = definitions.len();
        let mut registry = Self {
            pointer_type,
            pointer_size: pointer_type.bytes(),
            definitions,
            layouts: Vec::with_capacity(capacity),
            ids,
        };
        for definition in registry.definitions.clone() {
            registry.layouts.push(registry.layout_for(&definition, &mut Vec::new())?);
        }
        Ok(registry)
    }

    pub(super) fn id_for(&self, name: &str) -> Option<usize> {
        self.ids.get(name).copied()
    }

    pub(super) fn get(&self, id: usize) -> Option<&StructLayout> {
        self.layouts.get(id)
    }

    pub(super) fn type_for_name(&self, name: &str) -> Option<NativeType> {
        self.id_for(name).map(NativeType::Struct)
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
            fields.push(FieldLayout { name: field.name.clone(), offset, ty });
            offset += size;
            alignment = alignment.max(field_alignment);
        }
        visiting.pop();
        Ok(StructLayout { size: align_up(offset, alignment), alignment, fields })
    }

    fn native_type(
        &self,
        name: &str,
        visiting: &mut Vec<String>,
    ) -> Result<NativeType, NativeEmitError> {
        if let Some(ty) = lookup_builtin_type(name) {
            return match ty {
                BuiltinType::Int => Ok(NativeType::Int),
                BuiltinType::String => Ok(NativeType::String),
                BuiltinType::Buffer => Ok(NativeType::Buffer),
                BuiltinType::Array | BuiltinType::Map => {
                    Err(NativeEmitError(format!("unsupported layout type `{name}`")))
                }
            };
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

    fn type_layout(&self, ty: NativeType) -> Result<(u32, u32), NativeEmitError> {
        Ok(match ty {
            NativeType::Int => (4, 4),
            NativeType::String | NativeType::Buffer => (self.pointer_size, self.pointer_size),
            NativeType::Struct(id) => {
                let definition = self.definitions.get(id).ok_or_else(|| {
                    NativeEmitError(format!("missing nested struct layout `{id}`"))
                })?;
                let layout = self.layout_for(definition, &mut Vec::new())?;
                (layout.size, layout.alignment)
            }
        })
    }
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
}

use cranelift_codegen::ir::Type;

use crate::ast::{BuiltinType, PrimitiveType, TypeName, lookup_builtin_type, primitive_type};

use super::native::NativeEmitError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeType {
    Int,
    Integer { signed: bool, width: u8 },
    Float { width: u8 },
    Void,
    String,
    Buffer,
    Struct(usize),
    Pack(usize),
    Array(usize),
    Arena(u32),
    Enum(usize),
    FatPointer,
}

impl NativeType {
    pub(super) fn uses_indirect_ins(self) -> bool {
        matches!(
            self,
            Self::Int | Self::Integer { width: 1..=64, .. } | Self::Float { .. } | Self::Pack(_)
        )
    }

    pub(super) fn uses_sret(self) -> bool {
        matches!(self, Self::Struct(_) | Self::Integer { width: 65..=128, .. })
    }

    pub(super) fn is_wide_integer(self) -> bool {
        matches!(self, Self::Integer { width: 65..=128, .. })
    }

    pub(super) fn from_name(name: &str) -> Option<Self> {
        if let Some(capacity) = name.strip_prefix("Arena[").and_then(|name| name.strip_suffix(']'))
        {
            return capacity.parse::<u32>().ok().map(Self::Arena);
        }
        if let Some(primitive) = primitive_type(name) {
            return Some(Self::from_primitive(primitive));
        }
        match lookup_builtin_type(name)? {
            BuiltinType::Int => Some(Self::Int),
            BuiltinType::Bool => Some(Self::Int),
            BuiltinType::String => Some(Self::String),
            BuiltinType::Buffer => Some(Self::Buffer),
            BuiltinType::Array | BuiltinType::Map => None,
        }
    }

    pub(super) fn from_primitive(primitive: PrimitiveType) -> Self {
        match primitive {
            PrimitiveType::Integer { signed, width } => Self::Integer { signed, width },
            PrimitiveType::Float { width } => Self::Float { width },
            PrimitiveType::Void => Self::Void,
        }
    }

    pub(super) fn from_name_with_layout(
        name: &str,
        layouts: &super::layout::LayoutRegistry,
    ) -> Option<Self> {
        Self::from_name(name)
            .or_else(|| layouts.id_for(name).map(Self::Struct))
            .or_else(|| layouts.enum_id_for(name).map(Self::Enum))
    }

    pub(super) fn from_type_name_with_layout(
        type_name: Option<&TypeName>,
        layouts: &super::layout::LayoutRegistry,
    ) -> Result<Self, NativeEmitError> {
        let Some(type_name) = type_name else { return Ok(Self::Void) };
        Self::try_from_type_name_with_layout(Some(type_name), layouts)
            .ok_or_else(|| NativeEmitError(format!("unknown native type `{}`", type_name.name)))
    }

    pub(super) fn try_from_type_name_with_layout(
        type_name: Option<&TypeName>,
        layouts: &super::layout::LayoutRegistry,
    ) -> Option<Self> {
        type_name.and_then(|type_name| {
            if type_name.name == "Arena" {
                return type_name
                    .arguments
                    .first()
                    .and_then(|capacity| capacity.name.parse::<u32>().ok())
                    .map(Self::Arena);
            }
            layouts
                .type_for_type_name(type_name)
                .or_else(|| primitive_type(&type_name.name).map(Self::from_primitive))
                .or_else(|| Self::from_name_with_layout(&type_name.name, layouts))
        })
    }

    pub(super) fn ir_type(self, pointer_type: Type) -> Result<Type, NativeEmitError> {
        match self {
            Self::Int => Ok(cranelift_codegen::ir::types::I32),
            Self::Integer { width, .. } => integer_ir_type(width),
            Self::Float { width: 32 } => Ok(cranelift_codegen::ir::types::F32),
            Self::Float { width: 64 } => Ok(cranelift_codegen::ir::types::F64),
            Self::Float { width } => {
                Err(NativeEmitError(format!("invalid native float width `{width}`")))
            }
            Self::Void => Ok(cranelift_codegen::ir::types::I8),
            Self::String
            | Self::Buffer
            | Self::Struct(_)
            | Self::Enum(_)
            | Self::Pack(_)
            | Self::Array(_)
            | Self::Arena(_)
            | Self::FatPointer => Ok(pointer_type),
        }
    }
}

fn integer_ir_type(width: u8) -> Result<Type, NativeEmitError> {
    match width {
        1..=8 => Ok(cranelift_codegen::ir::types::I8),
        9..=16 => Ok(cranelift_codegen::ir::types::I16),
        17..=32 => Ok(cranelift_codegen::ir::types::I32),
        33..=64 => Ok(cranelift_codegen::ir::types::I64),
        65..=128 => Ok(cranelift_codegen::ir::types::I128),
        _ => Err(NativeEmitError(format!("invalid native integer width `{width}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::NativeType;
    use cranelift_codegen::ir::types;

    #[test]
    fn maps_primitive_widths_to_deterministic_machine_types() {
        assert_eq!(
            NativeType::Integer { signed: false, width: 8 }.ir_type(types::I64).unwrap(),
            types::I8
        );
        assert_eq!(
            NativeType::Integer { signed: true, width: 16 }.ir_type(types::I64).unwrap(),
            types::I16
        );
        assert_eq!(
            NativeType::Integer { signed: false, width: 32 }.ir_type(types::I64).unwrap(),
            types::I32
        );
        assert_eq!(
            NativeType::Integer { signed: true, width: 64 }.ir_type(types::I64).unwrap(),
            types::I64
        );
        assert_eq!(
            NativeType::Integer { signed: false, width: 128 }.ir_type(types::I64).unwrap(),
            types::I128
        );
        assert_eq!(NativeType::Float { width: 32 }.ir_type(types::I64).unwrap(), types::F32);
        assert_eq!(NativeType::Float { width: 64 }.ir_type(types::I64).unwrap(), types::F64);
        assert_eq!(NativeType::Void.ir_type(types::I64).unwrap(), types::I8);
    }
}

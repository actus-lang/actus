use cranelift_codegen::ir::Type;

use crate::ast::{BuiltinType, TypeName, lookup_builtin_type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeType {
    Int,
    String,
    Buffer,
}

impl NativeType {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        match lookup_builtin_type(name)? {
            BuiltinType::Int => Some(Self::Int),
            BuiltinType::String => Some(Self::String),
            BuiltinType::Buffer => Some(Self::Buffer),
            BuiltinType::Array | BuiltinType::Map => None,
        }
    }

    pub(super) fn from_type_name(type_name: Option<&TypeName>) -> Self {
        type_name.and_then(|type_name| Self::from_name(&type_name.name)).unwrap_or(Self::Int)
    }

    pub(super) fn ir_type(self, pointer_type: Type) -> Type {
        match self {
            Self::Int => cranelift_codegen::ir::types::I32,
            Self::String | Self::Buffer => pointer_type,
        }
    }
}

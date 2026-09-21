use cranelift_codegen::ir::Type;

use crate::ast::TypeName;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeType {
    Int,
    Buffer,
}

impl NativeType {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        match name {
            "Int" => Some(Self::Int),
            "Buffer" => Some(Self::Buffer),
            _ => None,
        }
    }

    pub(super) fn from_type_name(type_name: Option<&TypeName>) -> Self {
        type_name.and_then(|type_name| Self::from_name(&type_name.name)).unwrap_or(Self::Int)
    }

    pub(super) fn ir_type(self, pointer_type: Type) -> Type {
        match self {
            Self::Int => cranelift_codegen::ir::types::I32,
            Self::Buffer => pointer_type,
        }
    }
}

use cranelift_codegen::ir::Type;

use crate::ast::{BuiltinType, TypeName, lookup_builtin_type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeType {
    Int,
    String,
    Buffer,
    Struct(usize),
}

impl NativeType {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        match lookup_builtin_type(name)? {
            BuiltinType::Int => Some(Self::Int),
            BuiltinType::Bool => Some(Self::Int),
            BuiltinType::String => Some(Self::String),
            BuiltinType::Buffer => Some(Self::Buffer),
            BuiltinType::Array | BuiltinType::Map => None,
        }
    }

    pub(super) fn from_name_with_layout(
        name: &str,
        layouts: &super::layout::LayoutRegistry,
    ) -> Option<Self> {
        Self::from_name(name).or_else(|| layouts.id_for(name).map(Self::Struct))
    }

    pub(super) fn from_type_name_with_layout(
        type_name: Option<&TypeName>,
        layouts: &super::layout::LayoutRegistry,
    ) -> Self {
        type_name
            .and_then(|type_name| Self::from_name_with_layout(&type_name.name, layouts))
            .unwrap_or(Self::Int)
    }

    pub(super) fn ir_type(self, pointer_type: Type) -> Type {
        match self {
            Self::Int => cranelift_codegen::ir::types::I32,
            Self::String | Self::Buffer | Self::Struct(_) => pointer_type,
        }
    }
}

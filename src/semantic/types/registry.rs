use crate::ast::{BuiltinType, PrimitiveType, lookup_builtin_type, primitive_type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticType {
    Builtin(BuiltinType),
    Primitive(PrimitiveType),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TypeRegistry;

impl TypeRegistry {
    pub const fn new() -> Self {
        Self
    }

    pub fn resolve(&self, name: &str) -> Option<SemanticType> {
        primitive_type(name)
            .map(SemanticType::Primitive)
            .or_else(|| lookup_builtin_type(name).map(SemanticType::Builtin))
    }

    pub fn is_known(&self, name: &str) -> bool {
        self.resolve(name).is_some()
    }
}

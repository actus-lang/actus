use crate::ast::{EnumPayload, Expr, Role, TypeName};

use crate::semantic::analyzer::Analyzer;

/// Nesting depth at which a recursive declaration is assumed not to hold a
/// resource, so a self-referential struct or enum cannot loop forever.
const RESOURCE_SEARCH_DEPTH: usize = 8;

impl Analyzer {
    /// Whether a value of this type owns a cleanup obligation (ADR-0059): a
    /// buffer, an arena, a type with a drop contract, or an array, struct,
    /// enum, or generic instance that contains one. A view (`abs` or `ins`
    /// reference) never owns. Plain data aggregates are not resources.
    pub(in crate::semantic) fn type_holds_resource(&self, type_name: &TypeName) -> bool {
        self.type_holds_resource_within(type_name, 0)
    }

    /// Whether an argument or stored value holds a resource, judged by the
    /// declared type it is received as or, for a binding, by its own type.
    pub(in crate::semantic) fn argument_holds_resource(
        &self,
        expression: &Expr,
        declared: &TypeName,
    ) -> bool {
        if self.type_holds_resource(declared) {
            return true;
        }
        let Expr::Identifier { name, span } = expression else { return false };
        self.binding(name, *span)
            .ok()
            .and_then(|index| self.binding_type_names.get(&index))
            .is_some_and(|type_name| self.type_holds_resource(type_name))
    }

    fn type_holds_resource_within(&self, type_name: &TypeName, depth: usize) -> bool {
        if depth > RESOURCE_SEARCH_DEPTH
            || matches!(type_name.reference_role, Some(Role::Abs | Role::Ins))
        {
            return false;
        }
        let name = type_name.name.as_str();
        if matches!(name, "Buffer" | "Arena") || self.drop_types.contains(name) {
            return true;
        }
        if type_name
            .arguments
            .iter()
            .any(|argument| self.type_holds_resource_within(argument, depth + 1))
        {
            return true;
        }
        if let Some(definition) = self.struct_types.get(name) {
            return definition
                .fields
                .iter()
                .any(|field| self.type_holds_resource_within(&field.ty, depth + 1));
        }
        let Some(definition) = self.enum_types.get(name) else { return false };
        definition.variants.iter().any(|variant| match &variant.payload {
            EnumPayload::Unit => false,
            EnumPayload::Tuple(types) => {
                types.iter().any(|ty| self.type_holds_resource_within(ty, depth + 1))
            }
            EnumPayload::Struct(fields) => {
                fields.iter().any(|field| self.type_holds_resource_within(&field.ty, depth + 1))
            }
        })
    }
}

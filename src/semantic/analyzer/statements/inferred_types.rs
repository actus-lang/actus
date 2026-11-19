use crate::ast::{Expr, TypeName};

use super::super::Analyzer;

impl Analyzer {
    pub(super) fn record_inferred_binding_type(
        &mut self,
        binding_index: usize,
        initializer: &Expr,
        span: crate::lexer::SourceSpan,
    ) {
        if let Some(type_name) = self.resolved_type_name(initializer) {
            self.binding_type_names.insert(binding_index, type_name);
            return;
        }
        let Some(type_name) = self.expression_type_name(initializer) else { return };
        let Some(type_name) = super::super::calls::parse_type_name_key(&type_name, span) else {
            return;
        };
        if is_inferred_builtin_type(&type_name.name) {
            let arguments =
                type_name.arguments.into_iter().map(mark_inferred_builtin_types).collect();
            let mut type_name = TypeName::builtin(
                type_name.name,
                arguments,
                type_name.reference_role,
                type_name.span,
            );
            self.normalize_array_capacities(&mut type_name).ok();
            self.binding_type_names.insert(binding_index, type_name);
            return;
        }
        self.binding_type_names
            .insert(binding_index, self.normalize_imported_type_name(&type_name));
    }
}

fn is_inferred_builtin_type(name: &str) -> bool {
    crate::ast::lookup_builtin_type(name).is_some()
        || crate::ast::primitive_type(name).is_some()
        || matches!(name, "Option" | "Result" | "Arena")
}

fn mark_inferred_builtin_types(mut type_name: TypeName) -> TypeName {
    let arguments = type_name.arguments.into_iter().map(mark_inferred_builtin_types).collect();
    if is_inferred_builtin_type(&type_name.name) {
        return TypeName::builtin(
            type_name.name,
            arguments,
            type_name.reference_role,
            type_name.span,
        );
    }
    type_name.arguments = arguments;
    type_name
}

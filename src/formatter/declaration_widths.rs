use super::role_name;

pub(super) fn parameter_width(parameter: &crate::ast::Param) -> usize {
    role_name(&parameter.role).len() + parameter.name.len() + 4 + type_name_width(&parameter.ty)
}

fn type_name_width(type_name: &crate::ast::TypeName) -> usize {
    let role_width = type_name.reference_role.as_ref().map_or(0, |role| role_name(role).len() + 1);
    role_width
        + type_name.name.len()
        + if type_name.arguments.is_empty() {
            0
        } else {
            2 + type_name.arguments.iter().map(type_name_width).sum::<usize>()
                + 2 * type_name.arguments.len().saturating_sub(1)
        }
}

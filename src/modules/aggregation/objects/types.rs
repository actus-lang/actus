use crate::ast::TopLevelDecl;

pub(super) fn declaration_type_dependencies(
    declaration: &TopLevelDecl,
) -> Vec<&crate::ast::TypeName> {
    match declaration {
        TopLevelDecl::Struct(definition) => {
            definition.fields.iter().map(|field| &field.ty).collect()
        }
        TopLevelDecl::Enum(definition) => definition
            .variants
            .iter()
            .flat_map(|variant| match &variant.payload {
                crate::ast::EnumPayload::Unit => Vec::new(),
                crate::ast::EnumPayload::Tuple(fields) => fields.iter().collect(),
                crate::ast::EnumPayload::Struct(fields) => {
                    fields.iter().map(|field| &field.ty).collect()
                }
            })
            .collect(),
        TopLevelDecl::Pack(definition) => {
            let mut dependencies = vec![definition.storage.type_name()];
            dependencies.extend(definition.fields.iter().map(|field| &field.ty));
            dependencies
        }
        _ => Vec::new(),
    }
}

pub(super) fn type_declaration_identity(declaration: &TopLevelDecl) -> Option<(String, String)> {
    match declaration {
        TopLevelDecl::Struct(definition) => Some(("struct".to_owned(), definition.name.clone())),
        TopLevelDecl::Enum(definition) => Some(("enum".to_owned(), definition.name.clone())),
        TopLevelDecl::Pack(definition) => Some(("pack".to_owned(), definition.name.clone())),
        TopLevelDecl::Role(definition) => Some(("role".to_owned(), definition.name.clone())),
        _ => None,
    }
}

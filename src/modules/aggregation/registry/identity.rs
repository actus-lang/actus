use super::declarations::DeclarationRegistry;
use crate::ast::{DeclarationIdentity, TopLevelDecl};

pub(crate) const ROOT_MODULE_PATH: &str = "<root>";
pub(crate) const STANDALONE_PACKAGE_IDENTITY: &str = "<standalone>";

pub(crate) fn standalone_module_path() -> &'static str {
    ROOT_MODULE_PATH
}

impl DeclarationRegistry {
    /// Interned identities of the declarations of one program, by name.
    /// Code generation uses this to name a program's own declarations; it
    /// neither copies the program nor resolves spellings.
    pub(crate) fn declaration_ids(
        package: &str,
        module_path: &str,
        program: &crate::ast::Program,
    ) -> std::collections::BTreeMap<String, crate::ast::DeclarationId> {
        program
            .declarations
            .iter()
            .filter_map(declaration_name)
            .map(|name| {
                let identity = DeclarationIdentity::Declaration(crate::ast::DeclarationPath::new(
                    super::RegistryToken::new(),
                    package,
                    module_path,
                    name,
                ));
                (name.to_owned(), crate::ast::DeclarationId::intern(identity))
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn identity_for(
        &self,
        module_path: &str,
        name: &str,
    ) -> Option<DeclarationIdentity> {
        self.index
            .get(&(module_path.to_owned(), name.to_owned()))
            .and_then(|identities| identities.first().cloned())
    }
}

pub(crate) fn declaration_name(declaration: &TopLevelDecl) -> Option<&str> {
    match declaration {
        TopLevelDecl::Struct(value) => Some(&value.name),
        TopLevelDecl::Enum(value) => Some(&value.name),
        TopLevelDecl::Pack(value) => Some(&value.name),
        TopLevelDecl::Role(value) => Some(&value.name),
        TopLevelDecl::Verb(value) => Some(&value.name),
        TopLevelDecl::ExternalVerb(value) => Some(&value.name),
        TopLevelDecl::Constant(value) => Some(&value.name),
        TopLevelDecl::Serialize(value) => Some(&value.name),
        TopLevelDecl::Perform(_) | TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_) => None,
    }
}

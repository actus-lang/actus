use crate::ast::{ExternalVerbDecl, ForeignAbi, Program, TopLevelDecl};

use super::super::identity::ModuleNamespace;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleObjectOwner {
    Root,
    Imported { module_path: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleObjectUnit {
    pub(super) owner: ModuleObjectOwner,
    pub(super) namespace: ModuleNamespace,
    pub(super) program: Program,
    pub(super) exported_verbs: Vec<String>,
}

pub(super) fn dependency_declaration(
    declaration: &TopLevelDecl,
    namespace: Option<&str>,
) -> TopLevelDecl {
    match declaration {
        TopLevelDecl::Verb(verb) => TopLevelDecl::ExternalVerb(ExternalVerbDecl {
            is_open: verb.is_open,
            doc: verb.doc.clone(),
            contract: verb.contract.clone(),
            unsafe_boundary: false,
            module_import: true,
            abi: ForeignAbi::C,
            metadata: verb.metadata.clone(),
            name: namespace
                .map_or_else(|| verb.name.clone(), |value| format!("{value}::{}", verb.name)),
            native_symbol: namespace.map(|_| verb.name.clone()),
            generic_parameters: verb.generic_parameters.clone(),
            params: verb.params.clone(),
            return_type: verb.return_type.clone(),
            span: verb.span,
            identity: verb.identity,
        }),
        TopLevelDecl::ExternalVerb(verb) => {
            let mut imported = verb.clone();
            imported.unsafe_boundary = false;
            imported.module_import = true;
            if let Some(namespace) = namespace {
                imported.native_symbol = Some(imported.name.clone());
                imported.name = format!("{namespace}::{}", imported.name);
            }
            TopLevelDecl::ExternalVerb(imported)
        }
        _ => declaration.clone(),
    }
}

impl ModuleObjectUnit {
    pub fn owner(&self) -> &ModuleObjectOwner {
        &self.owner
    }

    pub fn namespace(&self) -> &ModuleNamespace {
        &self.namespace
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    pub fn exported_verbs(&self) -> &[String] {
        &self.exported_verbs
    }
}

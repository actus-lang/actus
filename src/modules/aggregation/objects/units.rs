use crate::ast::Program;

use super::identity::ModuleNamespace;

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

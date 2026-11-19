use std::fs;
use std::path::{Path, PathBuf};

use super::resolver::ModuleResolver;
use crate::session::CompilationSession;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageRole {
    Root,
    Dependency,
    RuntimeStd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageSourceIdentity {
    identity: String,
    source_root: PathBuf,
    role: PackageRole,
}

impl PackageSourceIdentity {
    pub fn new(
        identity: impl Into<String>,
        source_root: impl Into<PathBuf>,
        role: PackageRole,
    ) -> Self {
        let source_root = source_root.into();
        let source_root = fs::canonicalize(&source_root).unwrap_or(source_root);
        Self { identity: identity.into(), source_root, role }
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    pub const fn role(&self) -> &PackageRole {
        &self.role
    }
}

impl ModuleResolver {
    pub(crate) fn with_session(mut self, session: CompilationSession) -> Self {
        self.session = Some(session);
        self
    }

    pub(crate) fn session(&self) -> Option<&CompilationSession> {
        self.session.as_ref()
    }

    pub(crate) fn with_package_identity(
        mut self,
        identity: impl Into<String>,
        source_root: impl Into<PathBuf>,
    ) -> Self {
        self.package_identity =
            PackageSourceIdentity::new(identity, source_root, PackageRole::Root);
        self
    }

    pub(crate) fn package_identity(&self) -> &PackageSourceIdentity {
        &self.package_identity
    }

    pub(crate) fn module_package_identity(&self, module_path: &str) -> String {
        if module_path == "std" || module_path.starts_with("std::") {
            return "std".to_owned();
        }
        let root = module_path.split("::").next().unwrap_or_default();
        if self.dependency_roots.contains_key(root) {
            return root.replace('-', "_");
        }
        self.package_identity.identity.clone()
    }
}

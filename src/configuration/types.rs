use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use super::manifest;
use super::manifest::{BuildProfile, LibraryKind, OptimizationLevel};
use crate::target::{EntryContract, LinkerFlavor, TargetSpec};

pub(super) const LINKER_ENVIRONMENT_VARIABLE: &str = "ACTUS_LINKER";
pub(super) const DEFAULT_RUN_ARTIFACT_PREFIX: &str = "actus-run";
pub(super) const DEFAULT_NATIVE_MODULE_NAME: &str = "actus";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug)]
pub struct ConfigurationError(pub(super) String);

impl Display for ConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConfigurationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkLibrary {
    name: String,
    kind: LibraryKind,
}

#[derive(Clone, Debug)]
pub struct NativeBackendConfiguration {
    module_name: String,
    position_independent: bool,
    optimization_level: OptimizationLevel,
}

impl Default for NativeBackendConfiguration {
    fn default() -> Self {
        Self {
            module_name: DEFAULT_NATIVE_MODULE_NAME.to_owned(),
            position_independent: true,
            optimization_level: OptimizationLevel::None,
        }
    }
}

impl NativeBackendConfiguration {
    pub fn new(module_name: impl Into<String>, position_independent: bool) -> Self {
        Self {
            module_name: module_name.into(),
            position_independent,
            optimization_level: OptimizationLevel::None,
        }
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn position_independent(&self) -> bool {
        self.position_independent
    }

    pub fn with_optimization_level(mut self, optimization_level: OptimizationLevel) -> Self {
        self.optimization_level = optimization_level;
        self
    }

    pub const fn optimization_level(&self) -> OptimizationLevel {
        self.optimization_level
    }
}

#[derive(Clone, Debug)]
pub struct CompilerConfiguration {
    pub(super) project_root: PathBuf,
    pub(super) source_root: PathBuf,
    pub(super) dependency_roots: BTreeMap<String, PathBuf>,
    pub(super) target: TargetSpec,
    pub(super) target_spec_hash: String,
    pub(super) profile: BuildProfile,
    pub(super) profiles: manifest::ProfilesManifest,
    pub(super) linker: OsString,
    pub(super) linker_flavor: LinkerFlavor,
    pub(super) entry_contract: EntryContract,
    pub(super) run_artifact_prefix: String,
    pub(super) native_backend: NativeBackendConfiguration,
    pub(super) entry_symbol: Option<String>,
    pub(super) library_paths: Vec<PathBuf>,
    pub(super) libraries: Vec<LinkLibrary>,
}

impl LinkLibrary {
    pub(crate) fn new(name: String, kind: LibraryKind) -> Self {
        Self { name, kind }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn kind(&self) -> LibraryKind {
        self.kind
    }
}

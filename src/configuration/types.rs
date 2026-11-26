use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use super::manifest;
use super::manifest::{BuildProfile, LibraryKind, OptimizationLevel, RuntimeProfile};
use crate::codegen::BackendKind;
use crate::target::{
    EntryContract, LinkerFlavor, RelocationModel, TargetSpec, TimeProviderContract,
};

pub(super) const LINKER_ENVIRONMENT_VARIABLE: &str = "ACTUS_LINKER";
pub(super) const DEFAULT_RUN_ARTIFACT_PREFIX: &str = "actus-run";
pub(super) const DEFAULT_NATIVE_MODULE_NAME: &str = "actus";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageIdentity {
    pub name: String,
    pub version: String,
}

/// A configuration failure: what is wrong, its stable code, and where it is.
///
/// The code is part of the failure and not of the mode it was found in, so a
/// manifest error has the same code with and without `--strict` (ADR-0041).
/// A failure that names no code of its own is `E1800`. Where it is decides how
/// [`ConfigurationError::report`] prints it: in the manifest, at the key or
/// value it is about, or in the environment, with no file.
#[derive(Debug)]
pub struct ConfigurationError {
    pub(super) message: String,
    pub(super) code: Option<&'static str>,
    pub(super) place: ErrorPlace,
}

/// Where a configuration failure is.
#[derive(Debug, Default)]
pub(super) struct ErrorPlace {
    /// The manifest the failure is in; the loader of that manifest fills it in.
    pub(super) manifest: Option<PathBuf>,
    /// The failure is about the toolchain or the environment, not a file.
    pub(super) environment: bool,
    /// The dotted manifest key (`build.runtime`) the failure is about.
    pub(super) key: Option<&'static str>,
    /// The byte offset in the manifest, when the TOML parser gave one.
    pub(super) offset: Option<usize>,
}

impl ConfigurationError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), code: None, place: ErrorPlace::default() }
    }

    pub(super) fn coded(code: &'static str, message: impl Into<String>) -> Self {
        Self { message: message.into(), code: Some(code), place: ErrorPlace::default() }
    }

    /// A failure of the toolchain or the environment: it names no file.
    pub(super) fn environment(code: &'static str, message: impl Into<String>) -> Self {
        let place = ErrorPlace { environment: true, ..ErrorPlace::default() };
        Self { message: message.into(), code: Some(code), place }
    }

    /// Locates the failure at the value of a manifest key such as `build.runtime`.
    pub(super) fn at_key(mut self, key: &'static str) -> Self {
        self.place.key = Some(key);
        self
    }

    /// Locates the failure at a byte offset of the manifest.
    pub(super) fn at_offset(mut self, offset: Option<usize>) -> Self {
        self.place.offset = offset;
        self
    }

    /// Says that the failure is in `manifest`, unless it is already in a
    /// manifest or in the environment.
    pub(super) fn in_manifest(mut self, manifest: &std::path::Path) -> Self {
        if self.place.manifest.is_none() && !self.place.environment {
            self.place.manifest = Some(manifest.to_owned());
        }
        self
    }

    /// The stable diagnostic code of the failure.
    pub fn code(&self) -> &'static str {
        self.code.unwrap_or(crate::diagnostics::STRICT_CONFIGURATION_FAILURE)
    }
}

impl Display for ConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
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
    backend: BackendKind,
    module_name: String,
    position_independent: bool,
    optimization_level: OptimizationLevel,
    verify_no_float_ir: bool,
}

impl Default for NativeBackendConfiguration {
    fn default() -> Self {
        Self {
            backend: BackendKind::default(),
            module_name: DEFAULT_NATIVE_MODULE_NAME.to_owned(),
            position_independent: true,
            optimization_level: OptimizationLevel::None,
            verify_no_float_ir: false,
        }
    }
}

impl NativeBackendConfiguration {
    pub fn new(module_name: impl Into<String>, position_independent: bool) -> Self {
        Self {
            backend: BackendKind::default(),
            module_name: module_name.into(),
            position_independent,
            optimization_level: OptimizationLevel::None,
            verify_no_float_ir: false,
        }
    }

    pub const fn backend(&self) -> BackendKind {
        self.backend
    }

    pub const fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backend = backend;
        self
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn position_independent(&self) -> bool {
        self.position_independent
    }

    pub const fn relocation_model(&self) -> RelocationModel {
        if self.position_independent {
            RelocationModel::PositionIndependent
        } else {
            RelocationModel::Static
        }
    }

    pub fn with_optimization_level(mut self, optimization_level: OptimizationLevel) -> Self {
        self.optimization_level = optimization_level;
        self
    }

    pub const fn optimization_level(&self) -> OptimizationLevel {
        self.optimization_level
    }

    pub fn with_no_float_ir_verification(mut self) -> Self {
        self.verify_no_float_ir = true;
        self
    }

    pub const fn verifies_no_float_ir(&self) -> bool {
        self.verify_no_float_ir
    }
}

#[derive(Clone, Debug)]
pub struct CompilerConfiguration {
    pub(super) project_root: PathBuf,
    pub(super) source_root: PathBuf,
    pub(super) package_identity: String,
    pub(super) dependency_roots: BTreeMap<String, PathBuf>,
    pub(super) target: TargetSpec,
    pub(super) target_spec_hash: String,
    pub(super) lockfile_checksum: Option<String>,
    pub(super) time_provider: Option<TimeProviderContract>,
    pub(super) profile: BuildProfile,
    pub(super) profiles: manifest::ProfilesManifest,
    pub(super) linker: OsString,
    pub(super) linker_explicit: bool,
    pub(super) linker_script: Option<PathBuf>,
    pub(super) linker_flavor: LinkerFlavor,
    pub(super) entry_contract: EntryContract,
    pub(super) runtime: RuntimeProfile,
    pub(super) runtime_version: Option<String>,
    pub(super) runtime_checksum: Option<String>,
    pub(super) runtime_source_root: Option<PathBuf>,
    pub(super) runtime_module_roots: BTreeMap<String, PathBuf>,
    pub(super) run_artifact_prefix: String,
    pub(super) native_backend: NativeBackendConfiguration,
    pub(super) source_limits: manifest::SourceLimitMode,
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

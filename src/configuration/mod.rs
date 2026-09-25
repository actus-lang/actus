use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

pub use crate::target::{EntryContract, LinkerFlavor};
use crate::target::{TargetSpec, TargetSpecError};

mod lockfile;
mod manifest;

pub use lockfile::{ArcaLock, LockedPackage, LockfileError};
use manifest::ArcaManifest;
pub use manifest::OptimizationLevel;
pub use manifest::{BuildProfile, LibraryKind};

const LINKER_ENVIRONMENT_VARIABLE: &str = "ACTUS_LINKER";
const DEFAULT_RUN_ARTIFACT_PREFIX: &str = "actus-run";
const DEFAULT_NATIVE_MODULE_NAME: &str = "actus";

pub const HOSTED_ENTRY_SYMBOL: &str = "main";

#[derive(Debug)]
pub struct ConfigurationError(String);

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
    project_root: PathBuf,
    source_root: PathBuf,
    target: TargetSpec,
    target_spec_hash: String,
    profile: BuildProfile,
    profiles: manifest::ProfilesManifest,
    linker: OsString,
    linker_flavor: LinkerFlavor,
    entry_contract: EntryContract,
    run_artifact_prefix: String,
    native_backend: NativeBackendConfiguration,
    entry_symbol: Option<String>,
    library_paths: Vec<PathBuf>,
    libraries: Vec<LinkLibrary>,
}

impl CompilerConfiguration {
    pub fn from_environment() -> Self {
        let target = TargetSpec::host().expect("host target must have a valid target contract");
        let linker_flavor = target.linker_flavor();
        let entry_contract = target.entry_contract();
        let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
            .unwrap_or_else(|| OsString::from(target.default_linker()));
        Self {
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            source_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("src"),
            target_spec_hash: target.spec_hash(),
            target,
            profile: BuildProfile::Debug,
            profiles: manifest::ProfilesManifest::default(),
            linker,
            linker_flavor,
            entry_contract,
            run_artifact_prefix: DEFAULT_RUN_ARTIFACT_PREFIX.to_owned(),
            native_backend: NativeBackendConfiguration::default(),
            entry_symbol: None,
            library_paths: Vec::new(),
            libraries: Vec::new(),
        }
    }

    pub fn from_manifest(path: &Path) -> Result<Self, ConfigurationError> {
        let manifest = manifest::read(path)?;
        validate_lockfile(path)?;
        let environment = Self::from_environment();
        let (target, linker_flavor, entry_contract, linker) =
            resolve_manifest_target(&manifest, &environment)?;
        let manifest_directory = path.parent().unwrap_or_else(|| Path::new("."));
        let source_root = manifest::source_root(&manifest, manifest_directory);
        if manifest.package.source_root.is_some() && !source_root.is_dir() {
            return Err(ConfigurationError(format!(
                "InvalidSourceRoot: Arca.toml package.source_root `{}` does not exist",
                source_root.display()
            )));
        }
        let (profile, native_backend, libraries, library_paths) =
            build_settings(manifest.build, manifest.profile, &environment, manifest_directory);
        Ok(Self {
            project_root: manifest_directory.to_path_buf(),
            source_root,
            target_spec_hash: target.spec_hash(),
            target,
            profile,
            profiles: manifest.profile,
            linker,
            linker_flavor,
            entry_contract,
            native_backend,
            entry_symbol: manifest.package.entry,
            library_paths,
            libraries,
            ..environment
        })
    }

    pub fn from_current_manifest() -> Result<Self, ConfigurationError> {
        let path = Path::new(manifest::MANIFEST_FILE_NAME);
        if path.exists() { Self::from_manifest(path) } else { Ok(Self::from_environment()) }
    }

    pub fn from_input_path(path: &Path) -> Result<Self, ConfigurationError> {
        let start = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        let Some(manifest) = manifest::find_manifest(start) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest(&manifest)
    }

    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn linker(&self) -> &OsStr {
        &self.linker
    }

    pub const fn linker_flavor(&self) -> LinkerFlavor {
        self.linker_flavor
    }

    pub const fn entry_contract(&self) -> EntryContract {
        self.entry_contract
    }

    pub fn run_artifact_prefix(&self) -> &str {
        &self.run_artifact_prefix
    }

    pub fn native_backend(&self) -> &NativeBackendConfiguration {
        &self.native_backend
    }

    pub fn entry_symbol(&self) -> Option<&str> {
        self.entry_symbol.as_deref()
    }

    pub fn libraries(&self) -> &[LinkLibrary] {
        &self.libraries
    }

    pub fn library_paths(&self) -> &[PathBuf] {
        &self.library_paths
    }

    pub fn target(&self) -> &TargetSpec {
        &self.target
    }

    pub fn target_spec_hash(&self) -> &str {
        &self.target_spec_hash
    }

    pub const fn profile(&self) -> BuildProfile {
        self.profile
    }

    pub fn with_profile(mut self, profile: BuildProfile) -> Self {
        self.profile = profile;
        self.native_backend =
            self.native_backend.with_optimization_level(optimization_level(profile, self.profiles));
        self
    }

    pub fn capsula_target_directory(&self) -> PathBuf {
        self.project_root
            .join("capsula")
            .join(self.profile.directory_name())
            .join(self.target.triple().to_string())
    }
}

fn validate_lockfile(path: &Path) -> Result<(), ConfigurationError> {
    let lock_path = path.with_file_name("Arca.lock");
    if !lock_path.is_file() {
        return Ok(());
    }
    let lock_source = std::fs::read_to_string(&lock_path).map_err(|error| {
        ConfigurationError(format!("cannot read `{}`: {error}", lock_path.display()))
    })?;
    ArcaLock::parse(&lock_source)
        .and_then(|lock| lock.validate_against_manifest(path))
        .map_err(|error| ConfigurationError(error.to_string()))
}

fn build_settings(
    build: manifest::BuildManifest,
    profiles: manifest::ProfilesManifest,
    environment: &CompilerConfiguration,
    directory: &Path,
) -> (BuildProfile, NativeBackendConfiguration, Vec<LinkLibrary>, Vec<PathBuf>) {
    let profile = build.profile.unwrap_or(environment.profile);
    let native_backend = NativeBackendConfiguration::new(
        build.native_module.unwrap_or_else(|| environment.native_backend.module_name.clone()),
        build.position_independent.unwrap_or(environment.native_backend.position_independent),
    )
    .with_optimization_level(optimization_level(profile, profiles));
    let libraries = build
        .libraries
        .into_iter()
        .map(|library| LinkLibrary::new(library.name, library.kind))
        .collect();
    let library_paths = build.library_paths.into_iter().map(|path| directory.join(path)).collect();
    (profile, native_backend, libraries, library_paths)
}

fn optimization_level(
    profile: BuildProfile,
    profiles: manifest::ProfilesManifest,
) -> OptimizationLevel {
    let configured = match profile {
        BuildProfile::Debug => profiles.debug.opt_level,
        BuildProfile::Release => profiles.release.opt_level,
    };
    configured.unwrap_or(match profile {
        BuildProfile::Debug => OptimizationLevel::None,
        BuildProfile::Release => OptimizationLevel::Speed,
    })
}

fn resolve_manifest_target(
    manifest: &ArcaManifest,
    environment: &CompilerConfiguration,
) -> Result<(TargetSpec, LinkerFlavor, EntryContract, OsString), ConfigurationError> {
    let target = manifest
        .build
        .target
        .as_deref()
        .map(TargetSpec::parse)
        .transpose()
        .map_err(|error: TargetSpecError| ConfigurationError(error.to_string()))?
        .unwrap_or_else(|| environment.target.clone());
    let linker_flavor = manifest.build.linker_flavor.unwrap_or(target.linker_flavor());
    let entry_contract = manifest.build.entry_contract.unwrap_or_else(|| target.entry_contract());
    let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
        .or_else(|| manifest.build.linker.as_deref().map(OsString::from))
        .unwrap_or_else(|| OsString::from(target.default_linker()));
    Ok((target, linker_flavor, entry_contract, linker))
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

use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use crate::target::LinkerFlavor;
use crate::target::{TargetSpec, TargetSpecError};

const LINKER_ENVIRONMENT_VARIABLE: &str = "ACTUS_LINKER";
const DEFAULT_LINKER: &str = "cc";
const DEFAULT_RUN_ARTIFACT_PREFIX: &str = "actus-run";
const DEFAULT_NATIVE_MODULE_NAME: &str = "actus";
const MANIFEST_FILE_NAME: &str = "Arca.toml";
const DEFAULT_EDITION: &str = "alpha";

pub const HOSTED_ENTRY_SYMBOL: &str = "main";

#[derive(Debug)]
pub struct ConfigurationError(String);

impl Display for ConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConfigurationError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArcaManifest {
    package: PackageManifest,
    #[serde(default)]
    build: BuildManifest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageManifest {
    name: String,
    version: String,
    edition: Option<String>,
    entry: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildManifest {
    target: Option<String>,
    profile: Option<BuildProfile>,
    linker: Option<String>,
    linker_flavor: Option<LinkerFlavor>,
    native_module: Option<String>,
    position_independent: Option<bool>,
    #[serde(default)]
    library_paths: Vec<String>,
    #[serde(default)]
    libraries: Vec<LibraryManifest>,
}

#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BuildProfile {
    Debug,
    Release,
}

impl BuildProfile {
    pub const fn directory_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryManifest {
    name: String,
    kind: LibraryKind,
}

#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LibraryKind {
    Static,
    Shared,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkLibrary {
    name: String,
    kind: LibraryKind,
}

#[derive(Clone, Debug)]
pub struct NativeBackendConfiguration {
    module_name: String,
    position_independent: bool,
}

impl Default for NativeBackendConfiguration {
    fn default() -> Self {
        Self { module_name: DEFAULT_NATIVE_MODULE_NAME.to_owned(), position_independent: true }
    }
}

impl NativeBackendConfiguration {
    pub fn new(module_name: impl Into<String>, position_independent: bool) -> Self {
        Self { module_name: module_name.into(), position_independent }
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn position_independent(&self) -> bool {
        self.position_independent
    }
}

#[derive(Clone, Debug)]
pub struct CompilerConfiguration {
    project_root: PathBuf,
    target: TargetSpec,
    target_spec_hash: String,
    profile: BuildProfile,
    linker: OsString,
    linker_flavor: LinkerFlavor,
    run_artifact_prefix: String,
    native_backend: NativeBackendConfiguration,
    entry_symbol: Option<String>,
    library_paths: Vec<PathBuf>,
    libraries: Vec<LinkLibrary>,
}

impl CompilerConfiguration {
    pub fn from_environment() -> Self {
        let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
            .unwrap_or_else(|| OsString::from(DEFAULT_LINKER));
        let target = TargetSpec::host().expect("host target must have a valid target contract");
        let linker_flavor = target.linker_flavor();
        Self {
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            target_spec_hash: target.spec_hash(),
            target,
            profile: BuildProfile::Debug,
            linker,
            linker_flavor,
            run_artifact_prefix: DEFAULT_RUN_ARTIFACT_PREFIX.to_owned(),
            native_backend: NativeBackendConfiguration::default(),
            entry_symbol: None,
            library_paths: Vec::new(),
            libraries: Vec::new(),
        }
    }

    pub fn from_manifest(path: &Path) -> Result<Self, ConfigurationError> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            ConfigurationError(format!("cannot read `{}`: {error}", path.display()))
        })?;
        let manifest = toml::from_str::<ArcaManifest>(&source).map_err(|error| {
            ConfigurationError(format!("cannot parse `{}`: {error}", path.display()))
        })?;
        validate_manifest(&manifest)?;

        let environment = Self::from_environment();
        let target = manifest
            .build
            .target
            .as_deref()
            .map(TargetSpec::parse)
            .transpose()
            .map_err(|error: TargetSpecError| ConfigurationError(error.to_string()))?
            .unwrap_or_else(|| environment.target.clone());
        let linker_flavor = manifest.build.linker_flavor.unwrap_or(target.linker_flavor());
        let manifest_directory = path.parent().unwrap_or_else(|| Path::new("."));
        let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
            .or_else(|| manifest.build.linker.as_deref().map(OsString::from));
        let native_backend = NativeBackendConfiguration::new(
            manifest
                .build
                .native_module
                .unwrap_or_else(|| environment.native_backend.module_name.clone()),
            manifest
                .build
                .position_independent
                .unwrap_or(environment.native_backend.position_independent),
        );
        let libraries = manifest
            .build
            .libraries
            .into_iter()
            .map(|library| LinkLibrary::new(library.name, library.kind))
            .collect();
        let library_paths = manifest
            .build
            .library_paths
            .into_iter()
            .map(|path| manifest_directory.join(path))
            .collect();
        Ok(Self {
            project_root: manifest_directory.to_path_buf(),
            target_spec_hash: target.spec_hash(),
            target,
            profile: manifest.build.profile.unwrap_or(environment.profile),
            linker: linker.unwrap_or_else(|| OsString::from(DEFAULT_LINKER)),
            linker_flavor,
            native_backend,
            entry_symbol: manifest.package.entry,
            library_paths,
            libraries,
            ..environment
        })
    }

    pub fn from_current_manifest() -> Result<Self, ConfigurationError> {
        let path = Path::new(MANIFEST_FILE_NAME);
        if path.exists() { Self::from_manifest(path) } else { Ok(Self::from_environment()) }
    }

    pub fn linker(&self) -> &OsStr {
        &self.linker
    }

    pub const fn linker_flavor(&self) -> LinkerFlavor {
        self.linker_flavor
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

    pub fn capsula_target_directory(&self) -> PathBuf {
        self.project_root
            .join("capsula")
            .join(self.profile.directory_name())
            .join(self.target.triple().to_string())
    }
}

fn validate_manifest(manifest: &ArcaManifest) -> Result<(), ConfigurationError> {
    if manifest.package.name.trim().is_empty() || manifest.package.version.trim().is_empty() {
        return Err(ConfigurationError(
            "Arca.toml package name and version must not be empty".to_owned(),
        ));
    }
    if manifest.package.edition.as_deref().unwrap_or(DEFAULT_EDITION) != DEFAULT_EDITION {
        return Err(ConfigurationError(format!(
            "unsupported Actus edition; expected `{DEFAULT_EDITION}`"
        )));
    }
    if manifest.package.entry.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigurationError("Arca.toml entry must not be empty".to_owned()));
    }
    if manifest.build.linker.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigurationError("Arca.toml linker must not be empty".to_owned()));
    }
    if manifest.build.native_module.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigurationError("Arca.toml native_module must not be empty".to_owned()));
    }
    if manifest.build.library_paths.iter().any(|path| path.trim().is_empty()) {
        return Err(ConfigurationError(
            "Arca.toml library_paths must not contain empty paths".to_owned(),
        ));
    }
    if let Some(library) = manifest
        .build
        .libraries
        .iter()
        .find(|library| library.name.trim().is_empty() || library.name.starts_with('-'))
    {
        return Err(ConfigurationError(format!(
            "Arca.toml library name `{}` must be non-empty and must not start with `-`",
            library.name
        )));
    }
    Ok(())
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

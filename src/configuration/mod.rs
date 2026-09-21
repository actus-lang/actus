use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::path::Path;

use serde::Deserialize;

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
    linker: Option<String>,
    native_module: Option<String>,
    position_independent: Option<bool>,
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
    linker: OsString,
    run_artifact_prefix: String,
    native_backend: NativeBackendConfiguration,
    entry_symbol: Option<String>,
}

impl CompilerConfiguration {
    pub fn from_environment() -> Self {
        let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
            .unwrap_or_else(|| OsString::from(DEFAULT_LINKER));
        Self {
            linker,
            run_artifact_prefix: DEFAULT_RUN_ARTIFACT_PREFIX.to_owned(),
            native_backend: NativeBackendConfiguration::default(),
            entry_symbol: None,
        }
    }

    pub fn from_manifest(path: &Path) -> Result<Self, ConfigurationError> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            ConfigurationError(format!("cannot read `{}`: {error}", path.display()))
        })?;
        let manifest = toml::from_str::<ArcaManifest>(&source).map_err(|error| {
            ConfigurationError(format!("cannot parse `{}`: {error}", path.display()))
        })?;
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

        let environment = Self::from_environment();
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
        Ok(Self {
            linker: linker.unwrap_or_else(|| OsString::from(DEFAULT_LINKER)),
            native_backend,
            entry_symbol: manifest.package.entry,
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

    pub fn run_artifact_prefix(&self) -> &str {
        &self.run_artifact_prefix
    }

    pub fn native_backend(&self) -> &NativeBackendConfiguration {
        &self.native_backend
    }

    pub fn entry_symbol(&self) -> Option<&str> {
        self.entry_symbol.as_deref()
    }
}

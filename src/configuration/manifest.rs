use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::ConfigurationError;
use super::dependencies::DependencySpec;

pub(crate) const MANIFEST_FILE_NAME: &str = "Actus.toml";
pub(crate) const LEGACY_MANIFEST_FILE_NAME: &str = "Arca.toml";
pub(crate) const DEFAULT_EDITION: &str = "alpha";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActusManifest {
    pub(crate) package: PackageManifest,
    #[serde(default)]
    pub(crate) dependencies: std::collections::BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub(crate) build: BuildManifest,
    #[serde(default)]
    pub(crate) profile: ProfilesManifest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageManifest {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) edition: Option<String>,
    pub(crate) entry: Option<String>,
    pub(crate) source_root: Option<String>,
    #[serde(default)]
    pub(crate) dependencies: std::collections::BTreeMap<String, String>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BuildManifest {
    pub(crate) target: Option<String>,
    pub(crate) profile: Option<BuildProfile>,
    pub(crate) linker: Option<String>,
    pub(crate) linker_flavor: Option<crate::target::LinkerFlavor>,
    pub(crate) entry_contract: Option<crate::target::EntryContract>,
    pub(crate) native_module: Option<String>,
    pub(crate) position_independent: Option<bool>,
    #[serde(default)]
    pub(crate) library_paths: Vec<String>,
    #[serde(default)]
    pub(crate) libraries: Vec<LibraryManifest>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfilesManifest {
    #[serde(default)]
    pub(crate) debug: ProfileManifest,
    #[serde(default)]
    pub(crate) release: ProfileManifest,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileManifest {
    pub(crate) opt_level: Option<OptimizationLevel>,
}

#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationLevel {
    None,
    Speed,
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
pub(crate) struct LibraryManifest {
    pub(crate) name: String,
    pub(crate) kind: LibraryKind,
}

#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LibraryKind {
    Static,
    Shared,
}

pub(crate) fn read(path: &Path) -> Result<ActusManifest, ConfigurationError> {
    warn_if_legacy_manifest(path);
    let source = std::fs::read_to_string(path).map_err(|error| {
        ConfigurationError(format!("cannot read `{}`: {error}", path.display()))
    })?;
    let manifest = toml::from_str::<ActusManifest>(&source).map_err(|error| {
        ConfigurationError(format!("cannot parse `{}`: {error}", path.display()))
    })?;
    validate(&manifest)?;
    Ok(manifest)
}

pub(crate) fn validate(manifest: &ActusManifest) -> Result<(), ConfigurationError> {
    if manifest.package.name.trim().is_empty() || manifest.package.version.trim().is_empty() {
        return Err(ConfigurationError(
            "Actus.toml package name and version must not be empty".to_owned(),
        ));
    }
    if manifest.package.edition.as_deref().unwrap_or(DEFAULT_EDITION) != DEFAULT_EDITION {
        return Err(ConfigurationError(format!(
            "unsupported Actus edition; expected `{DEFAULT_EDITION}`"
        )));
    }
    if manifest.package.entry.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigurationError("Actus.toml entry must not be empty".to_owned()));
    }
    if manifest.package.source_root.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigurationError(
            "Actus.toml package.source_root must not be empty".to_owned(),
        ));
    }
    if manifest.build.linker.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigurationError("Actus.toml linker must not be empty".to_owned()));
    }
    if manifest.build.native_module.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigurationError("Actus.toml native_module must not be empty".to_owned()));
    }
    if manifest.build.library_paths.iter().any(|path| path.trim().is_empty()) {
        return Err(ConfigurationError(
            "Actus.toml library_paths must not contain empty paths".to_owned(),
        ));
    }
    if let Some(library) = manifest
        .build
        .libraries
        .iter()
        .find(|library| library.name.trim().is_empty() || library.name.starts_with('-'))
    {
        return Err(ConfigurationError(format!(
            "Actus.toml library name `{}` must be non-empty and must not start with `-`",
            library.name
        )));
    }
    Ok(())
}

pub(crate) fn source_root(manifest: &ActusManifest, directory: &Path) -> PathBuf {
    directory.join(manifest.package.source_root.as_deref().unwrap_or("src"))
}

pub(crate) fn find_manifest(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(manifest_in_directory)
}

pub(crate) fn manifest_in_directory(directory: &Path) -> Option<PathBuf> {
    let canonical = directory.join(MANIFEST_FILE_NAME);
    if canonical.is_file() {
        return Some(canonical);
    }
    let legacy = directory.join(LEGACY_MANIFEST_FILE_NAME);
    legacy.is_file().then_some(legacy)
}

fn warn_if_legacy_manifest(path: &Path) {
    if path.file_name().and_then(|name| name.to_str()) == Some(LEGACY_MANIFEST_FILE_NAME) {
        eprintln!("warning: 'Arca.toml' is deprecated, please rename to 'Actus.toml'");
    }
}

use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::configuration::CompilerConfiguration;

const ACTMETA_FORMAT_VERSION: u32 = 1;

#[derive(Debug)]
pub struct BuildGraphError(String);

impl Display for BuildGraphError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for BuildGraphError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnitMetadata {
    format_version: u32,
    pub unit: String,
    pub compiler_version: String,
    pub toolchain_hash: String,
    pub target_triple: String,
    pub target_spec_hash: String,
    pub profile: String,
}

impl UnitMetadata {
    pub fn for_configuration(
        unit: impl Into<String>,
        configuration: &CompilerConfiguration,
    ) -> Self {
        Self {
            format_version: ACTMETA_FORMAT_VERSION,
            unit: unit.into(),
            compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
            toolchain_hash: option_env!("ACTUS_TOOLCHAIN_HASH").unwrap_or("development").to_owned(),
            target_triple: configuration.target().triple().to_string(),
            target_spec_hash: configuration.target_spec_hash().to_owned(),
            profile: configuration.profile().directory_name().to_owned(),
        }
    }

    fn matches_configuration(&self, configuration: &CompilerConfiguration) -> bool {
        self.format_version == ACTMETA_FORMAT_VERSION
            && self.compiler_version == env!("CARGO_PKG_VERSION")
            && self.toolchain_hash == option_env!("ACTUS_TOOLCHAIN_HASH").unwrap_or("development")
            && self.target_triple == configuration.target().triple().to_string()
            && self.target_spec_hash == configuration.target_spec_hash()
            && self.profile == configuration.profile().directory_name()
    }

    fn read(path: &Path) -> Result<Self, BuildGraphError> {
        let source = fs::read_to_string(path).map_err(|error| {
            BuildGraphError(format!("cannot read `{}`: {error}", path.display()))
        })?;
        toml::from_str(&source)
            .map_err(|error| BuildGraphError(format!("cannot parse `{}`: {error}", path.display())))
    }
}

pub fn metadata_path(artifact: &Path) -> PathBuf {
    artifact.with_extension("actmeta")
}

pub fn write_metadata(
    artifact: &Path,
    configuration: &CompilerConfiguration,
) -> Result<(), BuildGraphError> {
    let unit = artifact.file_stem().and_then(|name| name.to_str()).ok_or_else(|| {
        BuildGraphError(format!("artifact `{}` has no valid unit name", artifact.display()))
    })?;
    let path = metadata_path(artifact);
    let metadata = UnitMetadata::for_configuration(unit, configuration);
    let source = toml::to_string(&metadata)
        .map_err(|error| BuildGraphError(format!("cannot encode `{}`: {error}", path.display())))?;
    fs::write(&path, source)
        .map_err(|error| BuildGraphError(format!("cannot write `{}`: {error}", path.display())))
}

pub fn invalidate_stale_artifact(
    artifact: &Path,
    configuration: &CompilerConfiguration,
) -> Result<(), BuildGraphError> {
    if !artifact.starts_with(configuration.capsula_target_directory()) {
        return Ok(());
    }
    let metadata = metadata_path(artifact);
    let compatible = metadata.exists()
        && UnitMetadata::read(&metadata)
            .map(|value| value.matches_configuration(configuration))
            .unwrap_or(false);
    if compatible {
        return Ok(());
    }
    for path in generated_paths(artifact) {
        remove_if_present(&path)?;
    }
    Ok(())
}

fn generated_paths(artifact: &Path) -> [PathBuf; 4] {
    [
        artifact.to_owned(),
        artifact.with_extension("obj"),
        artifact.with_extension("bin"),
        metadata_path(artifact),
    ]
}

fn remove_if_present(path: &Path) -> Result<(), BuildGraphError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(BuildGraphError(format!("cannot invalidate `{}`: {error}", path.display())))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{UnitMetadata, invalidate_stale_artifact, metadata_path};
    use crate::configuration::CompilerConfiguration;

    #[test]
    fn metadata_contains_the_target_spec_hash() {
        let configuration = CompilerConfiguration::from_environment();
        let metadata = UnitMetadata::for_configuration("sample", &configuration);
        assert_eq!(metadata.target_spec_hash, configuration.target_spec_hash());
        assert_eq!(metadata.target_triple, configuration.target().triple().to_string());
    }

    #[test]
    fn writes_round_trippable_metadata_for_a_capsula_artifact() {
        let configuration = CompilerConfiguration::from_environment();
        let artifact = configuration
            .capsula_target_directory()
            .join(format!("metadata-{}.obj", std::process::id()));
        fs::create_dir_all(artifact.parent().expect("artifact parent"))
            .expect("create artifact dir");
        fs::write(&artifact, b"object").expect("write artifact");
        super::write_metadata(&artifact, &configuration).expect("write metadata");
        let metadata = UnitMetadata::read(&metadata_path(&artifact)).expect("read metadata");
        assert_eq!(metadata.unit, format!("metadata-{}", std::process::id()));
        assert_eq!(metadata.target_spec_hash, configuration.target_spec_hash());
        let _ = fs::remove_file(&artifact);
        let _ = fs::remove_file(metadata_path(&artifact));
    }

    #[test]
    fn stale_target_metadata_invalidates_generated_artifacts() {
        let configuration = CompilerConfiguration::from_environment();
        let artifact = configuration
            .capsula_target_directory()
            .join(format!("stale-{}.obj", std::process::id()));
        let metadata = metadata_path(&artifact);
        fs::create_dir_all(artifact.parent().expect("artifact parent"))
            .expect("create artifact dir");
        fs::write(&artifact, b"stale").expect("write artifact");
        fs::write(&metadata, "format_version = 1\ntarget_spec_hash = \"stale\"\n")
            .expect("write stale metadata");
        invalidate_stale_artifact(&artifact, &configuration).expect("invalidation should succeed");
        assert!(!artifact.exists());
        assert!(!metadata.exists());
    }
}

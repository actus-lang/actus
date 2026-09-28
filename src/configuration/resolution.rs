use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::manifest::{
    ActusManifest, BuildManifest, BuildProfile, OptimizationLevel, ProfilesManifest,
};
use super::types::{CompilerConfiguration, LinkLibrary, NativeBackendConfiguration};
use super::{ConfigurationError, EntryContract, LinkerFlavor};
use crate::target::{TargetSpec, TargetSpecError};

pub(super) fn build_settings(
    build: BuildManifest,
    profiles: ProfilesManifest,
    environment: &CompilerConfiguration,
    directory: &Path,
) -> (BuildProfile, NativeBackendConfiguration, Vec<LinkLibrary>, Vec<PathBuf>) {
    let profile = build.profile.unwrap_or(environment.profile);
    let native_backend = NativeBackendConfiguration::new(
        build.native_module.unwrap_or_else(|| environment.native_backend.module_name().to_owned()),
        build.position_independent.unwrap_or(environment.native_backend.position_independent()),
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

pub(super) fn optimization_level(
    profile: BuildProfile,
    profiles: ProfilesManifest,
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

pub(super) fn resolve_manifest_target(
    manifest: &ActusManifest,
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
    let linker = std::env::var_os(super::types::LINKER_ENVIRONMENT_VARIABLE)
        .or_else(|| manifest.build.linker.as_deref().map(OsString::from))
        .unwrap_or_else(|| default_linker(&target));
    Ok((target, linker_flavor, entry_contract, linker))
}

pub(super) fn default_linker(target: &TargetSpec) -> OsString {
    OsString::from(target.default_linker())
}

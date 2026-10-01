use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::manifest::{
    ActusManifest, BuildManifest, BuildProfile, OptimizationLevel, ProfilesManifest, RuntimeProfile,
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

pub(super) fn runtime_source_root(
    runtime: RuntimeProfile,
) -> Result<Option<PathBuf>, ConfigurationError> {
    if runtime != RuntimeProfile::Std {
        return Ok(None);
    }
    let candidates = builtin_root_candidates();
    candidates.into_iter().find(|path| path.is_dir()).map(Some).ok_or_else(|| {
        ConfigurationError(
            "runtime profile `std` cannot locate the compiler-owned standard library".to_owned(),
        )
    })
}

pub(super) fn runtime_module_roots(
    runtime: RuntimeProfile,
) -> Result<std::collections::BTreeMap<String, PathBuf>, ConfigurationError> {
    let Some(source_root) = runtime_source_root(runtime)? else {
        return Ok(std::collections::BTreeMap::new());
    };
    let package_root = source_root.parent().ok_or_else(|| {
        ConfigurationError("builtin standard-library root has no package parent".to_owned())
    })?;
    let manifest = super::manifest::read(&package_root.join("Actus.toml"))?;
    let Some(runtime_manifest) = manifest.runtime else {
        return Err(ConfigurationError(
            "builtin standard library is missing its runtime registry".to_owned(),
        ));
    };
    let mut roots = std::collections::BTreeMap::new();
    for module in runtime_manifest.modules {
        let module_root = package_root.join(module.path);
        if !module_root.is_dir() {
            return Err(ConfigurationError(format!(
                "builtin standard-library module `{}` path does not exist: {}",
                module.name,
                module_root.display()
            )));
        }
        roots.insert(module.name, module_root);
    }
    Ok(roots)
}

fn builtin_root_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(sysroot) = std::env::var_os("ACTUS_SYSROOT") {
        let root = PathBuf::from(sysroot);
        candidates.push(root.join("std/src"));
        candidates.push(root.join("library/std/src"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        candidates.push(parent.join("../lib/actus/std/src"));
        candidates.push(parent.join("../share/actus/std/src"));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std/src"));
    candidates
}

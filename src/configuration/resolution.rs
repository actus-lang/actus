use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::manifest::{
    ActusManifest, BuildManifest, BuildProfile, OptimizationLevel, ProfilesManifest, RuntimeProfile,
};
use super::types::{CompilerConfiguration, LinkLibrary, NativeBackendConfiguration};
use super::{ConfigurationError, EntryContract, LinkerFlavor};
use crate::diagnostics::{STRICT_BUNDLED_COMPONENT_MISSING, STRICT_LINKER_IDENTITY_MISMATCH};
use crate::target::{TargetSpec, TargetSpecError};

pub(super) fn build_settings(
    build: BuildManifest,
    profiles: ProfilesManifest,
    environment: &CompilerConfiguration,
    directory: &Path,
) -> (BuildProfile, NativeBackendConfiguration, Vec<LinkLibrary>, Vec<PathBuf>, Option<PathBuf>) {
    let profile = build.profile.unwrap_or(environment.profile);
    let verify_no_float_ir = build.verify_no_float_ir.unwrap_or(false);
    let native_backend = NativeBackendConfiguration::new(
        build.native_module.unwrap_or_else(|| environment.native_backend.module_name().to_owned()),
        build.position_independent.unwrap_or(environment.native_backend.position_independent()),
    )
    .with_backend(build.backend.unwrap_or(environment.native_backend.backend()))
    .with_optimization_level(optimization_level(profile, profiles));
    let native_backend = if verify_no_float_ir {
        native_backend.with_no_float_ir_verification()
    } else {
        native_backend
    };
    let libraries = build
        .libraries
        .into_iter()
        .map(|library| LinkLibrary::new(library.name, library.kind))
        .collect();
    let library_paths = build.library_paths.into_iter().map(|path| directory.join(path)).collect();
    let linker_script = build.linker_script.map(|path| directory.join(path));
    (profile, native_backend, libraries, library_paths, linker_script)
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
) -> Result<(TargetSpec, LinkerFlavor, EntryContract, OsString, bool), ConfigurationError> {
    let target = manifest
        .build
        .target
        .as_deref()
        .map(TargetSpec::parse)
        .transpose()
        .map_err(|error: TargetSpecError| ConfigurationError::new(error.to_string()))?
        .unwrap_or_else(|| environment.target.clone());
    let linker_flavor = manifest.build.linker_flavor.unwrap_or(target.linker_flavor());
    let entry_contract = manifest.build.entry_contract.unwrap_or_else(|| target.entry_contract());
    let environment_linker = std::env::var_os(super::types::LINKER_ENVIRONMENT_VARIABLE);
    let manifest_linker = manifest.build.linker.as_deref().map(OsString::from);
    let linker_explicit = environment_linker.is_some() || manifest_linker.is_some();
    let linker = if let Some(linker) = environment_linker {
        linker
    } else if let Some(linker) = manifest_linker {
        linker
    } else if cfg!(feature = "llvm")
        && entry_contract == EntryContract::Freestanding
        && target.arm32_profile().is_some()
    {
        bundled_component(&target, "ld.lld")?
    } else {
        default_linker(&target)
    };
    Ok((target, linker_flavor, entry_contract, linker, linker_explicit))
}

pub(crate) fn bundled_component(
    target: &TargetSpec,
    component: &str,
) -> Result<OsString, ConfigurationError> {
    let root = bundled_toolchain_root(target).ok_or_else(|| {
        ConfigurationError::coded(
            STRICT_BUNDLED_COMPONENT_MISSING,
            format!("bundled component `{component}` is missing for freestanding target `{}`; run scripts/bootstrap_llvm.sh or scripts/bootstrap_llvm.ps1", target.triple()),
        )
    })?;
    let candidate =
        root.join("lib/actus/bin").join(format!("{component}{}", std::env::consts::EXE_SUFFIX));
    if !candidate.is_file() {
        return Err(ConfigurationError::coded(
            STRICT_BUNDLED_COMPONENT_MISSING,
            format!(
                "bundled component `{component}` is missing for freestanding target `{}`; run scripts/bootstrap_llvm.sh or scripts/bootstrap_llvm.ps1",
                target.triple()
            ),
        ));
    }
    if component == "ld.lld" {
        validate_bundled_linker_version(&candidate)?;
    }
    Ok(candidate.into_os_string())
}

fn validate_bundled_linker_version(path: &Path) -> Result<(), ConfigurationError> {
    let output = std::process::Command::new(path).arg("--version").output().map_err(|error| {
        ConfigurationError::coded(
            STRICT_LINKER_IDENTITY_MISMATCH,
            format!("cannot verify bundled linker `{}`: {error}", path.display()),
        )
    })?;
    let version = String::from_utf8_lossy(&output.stdout);
    if output.status.success() && version.contains("LLD 23.1.2") {
        return Ok(());
    }
    Err(ConfigurationError::coded(
        STRICT_LINKER_IDENTITY_MISMATCH,
        format!("bundled linker `{}` is not LLVM LLD 23.1.2: {}", path.display(), version.trim()),
    ))
}

pub(crate) fn bundled_builtins(target: &TargetSpec) -> Result<PathBuf, ConfigurationError> {
    let root = bundled_toolchain_root(target).ok_or_else(|| {
        ConfigurationError::coded(
            STRICT_BUNDLED_COMPONENT_MISSING,
            format!("bundled component `libclang_rt.builtins.a` is missing for freestanding target `{}`; run scripts/bootstrap_llvm.sh or scripts/bootstrap_llvm.ps1", target.triple()),
        )
    })?;
    let manifest = root.join("share/actus/toolchain.json");
    let contents = std::fs::read_to_string(&manifest).map_err(|error| {
        ConfigurationError::coded(
            STRICT_BUNDLED_COMPONENT_MISSING,
            format!("cannot read bundled toolchain manifest `{}`: {error}", manifest.display()),
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&contents).map_err(|error| {
        ConfigurationError::coded(
            STRICT_BUNDLED_COMPONENT_MISSING,
            format!("cannot parse bundled toolchain manifest `{}`: {error}", manifest.display()),
        )
    })?;
    let target_name = target.triple().to_string();
    let relative = value
        .get("compiler_rt_builtins")
        .and_then(|entries| entries.get(&target_name))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ConfigurationError::coded(
                STRICT_LINKER_IDENTITY_MISMATCH,
                format!("bundled toolchain has no builtins entry for target `{target_name}`"),
            )
        })?;
    let builtins = root.join(relative);
    if !builtins.is_file() {
        return Err(ConfigurationError::coded(
            STRICT_BUNDLED_COMPONENT_MISSING,
            format!(
                "bundled component `libclang_rt.builtins.a` is missing at `{}`",
                builtins.display()
            ),
        ));
    }
    Ok(builtins)
}

fn bundled_toolchain_root(target: &TargetSpec) -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let install_root = executable.parent()?.join("..");
    if install_root.join("share/actus/toolchain.json").is_file() {
        return Some(install_root);
    }
    let config = option_env!("ACTUS_LLVM_CONFIG_PATH")?;
    let prefix = Path::new(config).parent()?.parent()?;
    (target.arm32_profile().is_some() && prefix.join("share/actus/toolchain.json").is_file())
        .then(|| prefix.to_path_buf())
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
        ConfigurationError::new(
            "runtime profile `std` cannot locate the compiler-owned standard library".to_owned(),
        )
    })
}

pub(super) fn runtime_identity(
    runtime: RuntimeProfile,
    source_root: Option<&Path>,
) -> Result<(Option<String>, Option<String>), ConfigurationError> {
    let Some(source_root) = source_root else {
        return Ok((None, None));
    };
    let package_root = source_root.parent().ok_or_else(|| {
        ConfigurationError::new("builtin standard-library root has no package parent".to_owned())
    })?;
    let manifest = super::manifest::read(&package_root.join("Actus.toml"))?;
    let checksum = super::checksum::content_checksum(package_root)?;
    if runtime != RuntimeProfile::Std {
        return Ok((None, None));
    }
    Ok((Some(manifest.package.version), Some(checksum)))
}

pub(super) fn runtime_module_roots(
    runtime: RuntimeProfile,
    entry_contract: EntryContract,
) -> Result<std::collections::BTreeMap<String, PathBuf>, ConfigurationError> {
    let Some(source_root) = runtime_source_root(runtime)? else {
        return Ok(std::collections::BTreeMap::new());
    };
    let package_root = source_root.parent().ok_or_else(|| {
        ConfigurationError::new("builtin standard-library root has no package parent".to_owned())
    })?;
    let manifest = super::manifest::read(&package_root.join("Actus.toml"))?;
    let Some(runtime_manifest) = manifest.runtime else {
        return Err(ConfigurationError::new(
            "builtin standard library is missing its runtime registry".to_owned(),
        ));
    };
    let mut roots = std::collections::BTreeMap::new();
    for module in runtime_manifest.modules {
        if !module.targets.iter().any(|target| target.applies_to(entry_contract)) {
            continue;
        }
        let module_root = package_root.join(module.path);
        if !module_root.is_dir() {
            return Err(ConfigurationError::new(format!(
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

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::validate_bundled_linker_version;

    #[cfg(unix)]
    #[test]
    fn rejects_a_bundled_linker_with_the_wrong_version() {
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join(format!("actus-fake-ld-{}", std::process::id()));
        std::fs::write(&path, "#!/bin/sh\nprintf 'LLD 22.1.0\\n'\n").expect("write fake linker");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("make fake linker executable");
        let error = validate_bundled_linker_version(&path).expect_err("wrong version must fail");
        assert_eq!(error.code, Some(crate::diagnostics::STRICT_LINKER_IDENTITY_MISMATCH));
        let _ = std::fs::remove_file(path);
    }
}

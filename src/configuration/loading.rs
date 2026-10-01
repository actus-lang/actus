use std::path::{Path, PathBuf};

use super::ActusLock;
use super::manifest;
use super::resolution::{build_settings, default_linker, resolve_manifest_target};
use super::types::{
    CompilerConfiguration, ConfigurationError, DEFAULT_RUN_ARTIFACT_PREFIX,
    LINKER_ENVIRONMENT_VARIABLE, PackageIdentity,
};

impl CompilerConfiguration {
    pub fn from_environment() -> Self {
        let target = crate::target::TargetSpec::host()
            .expect("host target must have a valid target contract");
        let linker_flavor = target.linker_flavor();
        let entry_contract = target.entry_contract();
        let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
            .unwrap_or_else(|| default_linker(&target));
        Self {
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            source_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("src"),
            dependency_roots: std::collections::BTreeMap::new(),
            target_spec_hash: target.spec_hash(),
            target,
            profile: super::BuildProfile::Debug,
            profiles: manifest::ProfilesManifest::default(),
            linker,
            linker_flavor,
            entry_contract,
            runtime: super::RuntimeProfile::Core,
            runtime_version: None,
            runtime_checksum: None,
            runtime_source_root: None,
            runtime_module_roots: std::collections::BTreeMap::new(),
            run_artifact_prefix: DEFAULT_RUN_ARTIFACT_PREFIX.to_owned(),
            native_backend: super::NativeBackendConfiguration::default(),
            source_limits: manifest::SourceLimitMode::default(),
            entry_symbol: None,
            library_paths: Vec::new(),
            libraries: Vec::new(),
        }
    }

    pub fn from_manifest(path: &Path) -> Result<Self, ConfigurationError> {
        Self::from_manifest_with_lockfile_policy(path, true, false)
    }

    pub fn from_manifest_read_only(path: &Path) -> Result<Self, ConfigurationError> {
        Self::from_manifest_with_lockfile_policy(path, false, false)
    }

    pub(crate) fn from_manifest_with_lockfile_policy(
        path: &Path,
        sync_missing_lockfile: bool,
        reject_legacy_manifest: bool,
    ) -> Result<Self, ConfigurationError> {
        let manifest = read_manifest_for_policy(path, reject_legacy_manifest)?;
        let dependency_graph = super::dependencies::resolve(path, reject_legacy_manifest)?;
        validate_lockfile(path, sync_missing_lockfile)?;
        let environment = Self::from_environment();
        build_from_manifest(path, manifest, dependency_graph, environment)
    }

    pub fn from_current_manifest() -> Result<Self, ConfigurationError> {
        let Some(path) = manifest::manifest_in_directory(Path::new(".")) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest(&path)
    }

    pub fn from_input_path(path: &Path) -> Result<Self, ConfigurationError> {
        let start = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        let Some(manifest) = manifest::find_manifest(start) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest(&manifest)
    }

    pub fn from_input_path_read_only(path: &Path) -> Result<Self, ConfigurationError> {
        let start = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        let Some(manifest) = manifest::find_manifest(start) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest_read_only(&manifest)
    }
}

fn build_from_manifest(
    path: &Path,
    manifest: manifest::ActusManifest,
    dependency_graph: super::dependencies::DependencyGraph,
    environment: CompilerConfiguration,
) -> Result<CompilerConfiguration, ConfigurationError> {
    let (target, linker_flavor, entry_contract, linker) =
        resolve_manifest_target(&manifest, &environment)?;
    let runtime = manifest.build.runtime.unwrap_or_default();
    validate_runtime_profile(runtime, entry_contract)?;
    validate_runtime_dependency_alias(runtime, &dependency_graph.roots)?;
    let runtime_source_root = super::resolution::runtime_source_root(runtime)?;
    let (runtime_version, runtime_checksum) =
        super::resolution::runtime_identity(runtime, runtime_source_root.as_deref())?;
    let runtime_module_roots = super::resolution::runtime_module_roots(runtime, entry_contract)?;
    let manifest_directory = path.parent().unwrap_or_else(|| Path::new("."));
    let source_root = validated_source_root(&manifest, manifest_directory)?;
    let (profile, native_backend, libraries, library_paths) =
        build_settings(manifest.build, manifest.profile, &environment, manifest_directory);
    Ok(CompilerConfiguration {
        project_root: manifest_directory.to_path_buf(),
        source_root,
        dependency_roots: dependency_graph.roots,
        target_spec_hash: target.spec_hash(),
        target,
        profile,
        profiles: manifest.profile,
        linker,
        linker_flavor,
        entry_contract,
        runtime,
        runtime_version,
        runtime_checksum,
        runtime_source_root,
        runtime_module_roots,
        native_backend,
        source_limits: manifest.package.source_limits.unwrap_or_default(),
        entry_symbol: manifest.package.entry,
        library_paths,
        libraries,
        ..environment
    })
}

fn validate_runtime_dependency_alias(
    runtime: manifest::RuntimeProfile,
    dependency_roots: &std::collections::BTreeMap<String, PathBuf>,
) -> Result<(), ConfigurationError> {
    if runtime == manifest::RuntimeProfile::Std && dependency_roots.contains_key("std") {
        return Err(ConfigurationError(
            "dependency alias `std` conflicts with the builtin standard library".to_owned(),
        ));
    }
    Ok(())
}

fn validate_runtime_profile(
    runtime: manifest::RuntimeProfile,
    entry_contract: crate::target::EntryContract,
) -> Result<(), ConfigurationError> {
    let compatible = match runtime {
        manifest::RuntimeProfile::Core => true,
        manifest::RuntimeProfile::Std => true,
        manifest::RuntimeProfile::Freestanding => {
            matches!(entry_contract, crate::target::EntryContract::Freestanding)
        }
    };
    if compatible {
        return Ok(());
    }
    let target_kind = match entry_contract {
        crate::target::EntryContract::Hosted => "hosted",
        crate::target::EntryContract::Freestanding => "freestanding",
    };
    Err(ConfigurationError(format!(
        "runtime profile `{runtime}` is incompatible with {target_kind} target"
    )))
}

fn validated_source_root(
    manifest: &manifest::ActusManifest,
    manifest_directory: &Path,
) -> Result<PathBuf, ConfigurationError> {
    let source_root = manifest::source_root(manifest, manifest_directory);
    if manifest.package.source_root.is_none() || source_root.is_dir() {
        return Ok(source_root);
    }
    Err(ConfigurationError(format!(
        "InvalidSourceRoot: Actus.toml package.source_root `{}` does not exist",
        source_root.display()
    )))
}

fn read_manifest_for_policy(
    path: &Path,
    reject_legacy_manifest: bool,
) -> Result<manifest::ActusManifest, ConfigurationError> {
    if reject_legacy_manifest && manifest::is_legacy_manifest(path) {
        return Err(ConfigurationError(
            "strict mode rejects deprecated `Arca.toml`; rename it to `Actus.toml`".to_owned(),
        ));
    }
    if !reject_legacy_manifest {
        manifest::warn_if_legacy_manifest(path);
    }
    manifest::read(path)
}

pub(crate) fn manifest_path_in_directory(path: &Path) -> Option<PathBuf> {
    manifest::manifest_in_directory(path)
}

pub fn package_identity(path: &Path) -> Result<PackageIdentity, ConfigurationError> {
    let manifest = manifest::read(path)?;
    Ok(PackageIdentity { name: manifest.package.name, version: manifest.package.version })
}

fn validate_lockfile(path: &Path, sync_missing_lockfile: bool) -> Result<(), ConfigurationError> {
    let lock_path = path.with_file_name("Actus.lock");
    if !lock_path.is_file() {
        if sync_missing_lockfile {
            ActusLock::sync(path).map_err(|error| ConfigurationError(error.to_string()))?;
        }
        return Ok(());
    }
    let lock_source = std::fs::read_to_string(&lock_path).map_err(|error| {
        ConfigurationError(format!("cannot read `{}`: {error}", lock_path.display()))
    })?;
    ActusLock::parse(&lock_source)
        .and_then(|lock| lock.validate_against_manifest(path))
        .map_err(|error| ConfigurationError(error.to_string()))
}

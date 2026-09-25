use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::ConfigurationError;
use super::manifest::{ArcaManifest, source_root};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub(crate) enum DependencySpec {
    Version(String),
    Local(DependencyTable),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependencyTable {
    pub(crate) path: Option<String>,
    pub(crate) version: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct DependencyPackage {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) path: Option<String>,
    pub(crate) checksum: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DependencyGraph {
    pub(crate) roots: BTreeMap<String, PathBuf>,
    pub(crate) packages: Vec<DependencyPackage>,
}

pub(crate) fn resolve(manifest_path: &Path) -> Result<DependencyGraph, ConfigurationError> {
    let mut graph = DependencyGraph::default();
    let mut visiting = HashSet::new();
    visit_manifest(manifest_path, &mut graph, &mut visiting)?;
    graph.packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(graph)
}

fn visit_manifest(
    manifest_path: &Path,
    graph: &mut DependencyGraph,
    visiting: &mut HashSet<PathBuf>,
) -> Result<(), ConfigurationError> {
    let canonical = fs::canonicalize(manifest_path).map_err(|error| {
        ConfigurationError(format!(
            "cannot resolve manifest `{}`: {error}",
            manifest_path.display()
        ))
    })?;
    if !visiting.insert(canonical.clone()) {
        return Err(ConfigurationError(format!(
            "DependencyCycle: dependency graph contains `{}`",
            canonical.display()
        )));
    }
    let manifest = super::manifest::read(&canonical)?;
    let dependencies = dependency_specs(&manifest)?;
    for (name, specification) in dependencies {
        match specification {
            DependencySpec::Version(version) => {
                super::version::validate_constraint(&version).map_err(|error| {
                    ConfigurationError(format!("InvalidVersionConstraint for `{name}`: {error}"))
                })?;
                record_package(
                    graph,
                    DependencyPackage { name, version, path: None, checksum: None },
                )?;
            }
            DependencySpec::Local(table) => {
                visit_local_dependency(&canonical, name, table, graph, visiting)?
            }
        }
    }
    visiting.remove(&canonical);
    Ok(())
}

fn dependency_specs(
    manifest: &ArcaManifest,
) -> Result<BTreeMap<String, DependencySpec>, ConfigurationError> {
    let mut dependencies = BTreeMap::new();
    for (name, version) in &manifest.package.dependencies {
        dependencies.insert(name.clone(), DependencySpec::Version(version.clone()));
    }
    for (name, specification) in &manifest.dependencies {
        if let Some(previous) = dependencies.get(name)
            && previous != specification
        {
            return Err(ConfigurationError(format!(
                "VersionConflict: dependency `{name}` has conflicting declarations"
            )));
        }
        dependencies.insert(name.clone(), specification.clone());
    }
    Ok(dependencies)
}

fn visit_local_dependency(
    parent_manifest: &Path,
    name: String,
    table: DependencyTable,
    graph: &mut DependencyGraph,
    visiting: &mut HashSet<PathBuf>,
) -> Result<(), ConfigurationError> {
    let relative_path = table.path.ok_or_else(|| {
        ConfigurationError(format!("dependency `{name}` must declare a local `path`"))
    })?;
    let package_root =
        parent_manifest.parent().unwrap_or_else(|| Path::new(".")).join(&relative_path);
    let dependency_manifest = package_root.join("Arca.toml");
    if !dependency_manifest.is_file() {
        return Err(ConfigurationError(format!(
            "InvalidDependencyPath: dependency `{name}` has no Arca.toml at `{}`",
            package_root.display()
        )));
    }
    let child = super::manifest::read(&dependency_manifest)?;
    if let Some(constraint) = &table.version {
        let constraint = super::version::VersionConstraint::parse(constraint).map_err(|error| {
            ConfigurationError(format!("InvalidVersionConstraint for `{name}`: {error}"))
        })?;
        let package_version = super::version::Version::parse(&child.package.version)
            .map_err(|error| ConfigurationError(format!("InvalidVersion for `{name}`: {error}")))?;
        if !constraint.matches(&package_version) {
            return Err(ConfigurationError(format!(
                "VersionConflict: dependency `{name}` requires `{}` but package provides `{}`",
                constraint_text(constraint),
                child.package.version
            )));
        }
    }
    let child_source_root = source_root(&child, &package_root);
    if !child_source_root.is_dir() {
        return Err(ConfigurationError(format!(
            "InvalidSourceRoot: dependency `{name}` source root `{}` does not exist",
            child_source_root.display()
        )));
    }
    graph.roots.insert(name.clone(), child_source_root);
    record_package(
        graph,
        DependencyPackage {
            name,
            version: child.package.version.clone(),
            path: Some(relative_path),
            checksum: Some(content_checksum(&package_root)?),
        },
    )?;
    visit_manifest(&dependency_manifest, graph, visiting)
}

fn record_package(
    graph: &mut DependencyGraph,
    package: DependencyPackage,
) -> Result<(), ConfigurationError> {
    if let Some(previous) = graph.packages.iter().find(|entry| entry.name == package.name) {
        if previous.version != package.version || previous.path != package.path {
            return Err(ConfigurationError(format!(
                "VersionConflict: dependency `{}` resolves to incompatible packages",
                package.name
            )));
        }
        return Ok(());
    }
    graph.packages.push(package);
    Ok(())
}

fn constraint_text(constraint: super::version::VersionConstraint) -> String {
    match constraint {
        super::version::VersionConstraint::Exact(version) => {
            format!("={}.{}.{}", version.major, version.minor, version.patch)
        }
        super::version::VersionConstraint::Compatible(version) => {
            format!("^{}.{}.{}", version.major, version.minor, version.patch)
        }
        super::version::VersionConstraint::GreaterOrEqual(version) => {
            format!(">={}.{}.{}", version.major, version.minor, version.patch)
        }
    }
}

fn content_checksum(root: &Path) -> Result<String, ConfigurationError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = 0xcbf29ce484222325_u64;
    for (relative, path) in files {
        update_hash(&mut hash, relative.as_bytes());
        update_hash(
            &mut hash,
            &fs::read(path).map_err(|error| {
                ConfigurationError(format!("cannot hash dependency content: {error}"))
            })?,
        );
    }
    Ok(format!("{hash:016x}"))
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), ConfigurationError> {
    let entries = fs::read_dir(directory).map_err(|error| {
        ConfigurationError(format!("cannot inspect dependency `{}`: {error}", directory.display()))
    })?;
    for entry in entries {
        let path = entry.map_err(|error| ConfigurationError(error.to_string()))?.path();
        if path.file_name().is_some_and(|name| matches!(name.to_str(), Some(".git" | "capsula"))) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let relative =
                path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            files.push((relative, path));
        }
    }
    Ok(())
}

fn update_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::ConfigurationError;
use super::checksum::content_checksum;
use super::manifest::{ActusManifest, source_root};

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

pub(crate) fn resolve(
    manifest_path: &Path,
    reject_legacy_manifest: bool,
) -> Result<DependencyGraph, ConfigurationError> {
    let mut graph = DependencyGraph::default();
    let mut visiting = HashSet::new();
    visit_manifest(manifest_path, &mut graph, &mut visiting, reject_legacy_manifest)?;
    graph.packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(graph)
}

fn visit_manifest(
    manifest_path: &Path,
    graph: &mut DependencyGraph,
    visiting: &mut HashSet<PathBuf>,
    reject_legacy_manifest: bool,
) -> Result<(), ConfigurationError> {
    reject_legacy_dependency(manifest_path, reject_legacy_manifest)?;
    let canonical = canonical_manifest_path(manifest_path)?;
    if !visiting.insert(canonical.clone()) {
        return Err(ConfigurationError(format!(
            "DependencyCycle: dependency graph contains `{}`",
            canonical.display()
        )));
    }
    let manifest = super::manifest::read(&canonical)?;
    let dependencies = dependency_specs(&manifest)?;
    visit_dependency_specs(&canonical, dependencies, graph, visiting, reject_legacy_manifest)?;
    visiting.remove(&canonical);
    Ok(())
}

fn reject_legacy_dependency(
    manifest_path: &Path,
    reject_legacy_manifest: bool,
) -> Result<(), ConfigurationError> {
    if reject_legacy_manifest && super::manifest::is_legacy_manifest(manifest_path) {
        return Err(ConfigurationError(
            "strict mode rejects deprecated dependency `Arca.toml`; rename it to `Actus.toml`"
                .to_owned(),
        ));
    }
    Ok(())
}

fn canonical_manifest_path(manifest_path: &Path) -> Result<PathBuf, ConfigurationError> {
    fs::canonicalize(manifest_path).map_err(|error| {
        ConfigurationError(format!(
            "cannot resolve manifest `{}`: {error}",
            manifest_path.display()
        ))
    })
}

fn visit_dependency_specs(
    canonical: &Path,
    dependencies: BTreeMap<String, DependencySpec>,
    graph: &mut DependencyGraph,
    visiting: &mut HashSet<PathBuf>,
    reject_legacy_manifest: bool,
) -> Result<(), ConfigurationError> {
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
            DependencySpec::Local(table) => visit_local_dependency(
                canonical,
                name,
                table,
                graph,
                visiting,
                reject_legacy_manifest,
            )?,
        }
    }
    Ok(())
}

fn dependency_specs(
    manifest: &ActusManifest,
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
    reject_legacy_manifest: bool,
) -> Result<(), ConfigurationError> {
    let relative_path = required_local_path(&name, table.path)?;
    let (package_root, dependency_manifest, child) =
        load_local_dependency(parent_manifest, &name, &relative_path, reject_legacy_manifest)?;
    validate_dependency_version(&name, table.version.as_deref(), &child.package.version)?;
    let child_source_root = dependency_source_root(&name, &child, &package_root)?;
    graph.roots.insert(name.clone(), child_source_root);
    record_local_package(graph, name, relative_path, &child, &package_root)?;
    visit_manifest(&dependency_manifest, graph, visiting, reject_legacy_manifest)
}

fn required_local_path(name: &str, path: Option<String>) -> Result<String, ConfigurationError> {
    path.ok_or_else(|| {
        ConfigurationError(format!("dependency `{name}` must declare a local `path`"))
    })
}

fn dependency_source_root(
    name: &str,
    child: &ActusManifest,
    package_root: &Path,
) -> Result<PathBuf, ConfigurationError> {
    let child_source_root = source_root(child, package_root);
    if child_source_root.is_dir() {
        return Ok(child_source_root);
    }
    Err(ConfigurationError(format!(
        "InvalidSourceRoot: dependency `{name}` source root `{}` does not exist",
        child_source_root.display()
    )))
}

fn record_local_package(
    graph: &mut DependencyGraph,
    name: String,
    relative_path: String,
    child: &ActusManifest,
    package_root: &Path,
) -> Result<(), ConfigurationError> {
    record_package(
        graph,
        DependencyPackage {
            name,
            version: child.package.version.clone(),
            path: Some(relative_path),
            checksum: Some(content_checksum(package_root)?),
        },
    )
}

fn load_local_dependency(
    parent_manifest: &Path,
    name: &str,
    relative_path: &str,
    reject_legacy_manifest: bool,
) -> Result<(PathBuf, PathBuf, ActusManifest), ConfigurationError> {
    let package_root =
        parent_manifest.parent().unwrap_or_else(|| Path::new(".")).join(relative_path);
    let Some(dependency_manifest) = super::manifest::manifest_in_directory(&package_root) else {
        return Err(ConfigurationError(format!(
            "InvalidDependencyPath: dependency `{name}` has no Actus.toml at `{}`",
            package_root.display()
        )));
    };
    if reject_legacy_manifest && super::manifest::is_legacy_manifest(&dependency_manifest) {
        return Err(ConfigurationError(
            "strict mode rejects deprecated dependency `Arca.toml`; rename it to `Actus.toml`"
                .to_owned(),
        ));
    }
    if !reject_legacy_manifest {
        super::manifest::warn_if_legacy_manifest(&dependency_manifest);
    }
    let child = super::manifest::read(&dependency_manifest)?;
    Ok((package_root, dependency_manifest, child))
}

fn validate_dependency_version(
    name: &str,
    constraint_text_value: Option<&str>,
    package_version_text: &str,
) -> Result<(), ConfigurationError> {
    let Some(constraint_text_value) = constraint_text_value else {
        return Ok(());
    };
    let constraint =
        super::version::VersionConstraint::parse(constraint_text_value).map_err(|error| {
            ConfigurationError(format!("InvalidVersionConstraint for `{name}`: {error}"))
        })?;
    let package_version = super::version::Version::parse(package_version_text)
        .map_err(|error| ConfigurationError(format!("InvalidVersion for `{name}`: {error}")))?;
    if constraint.matches(&package_version) {
        return Ok(());
    }
    Err(ConfigurationError(format!(
        "VersionConflict: dependency `{name}` requires `{}` but package provides `{package_version_text}`",
        constraint_text(constraint)
    )))
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

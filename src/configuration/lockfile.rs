use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const LOCKFILE_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActusLock {
    pub lockfile_version: u32,
    #[serde(rename = "package")]
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug)]
pub struct LockfileError(pub String);

impl std::fmt::Display for LockfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for LockfileError {}

impl ActusLock {
    pub fn from_packages(mut packages: Vec<LockedPackage>) -> Self {
        packages.sort_by(|left, right| {
            (&left.name, &left.version, &left.source, &left.path).cmp(&(
                &right.name,
                &right.version,
                &right.source,
                &right.path,
            ))
        });
        Self { lockfile_version: LOCKFILE_VERSION, packages }
    }

    pub fn serialize(&self) -> Result<String, LockfileError> {
        toml::to_string_pretty(self).map_err(|error| LockfileError(error.to_string()))
    }

    pub fn parse(source: &str) -> Result<Self, LockfileError> {
        let lockfile = toml::from_str::<Self>(source)
            .map_err(|error| LockfileError(format!("cannot parse Actus.lock: {error}")))?;
        lockfile.validate_shape()?;
        Ok(lockfile)
    }

    pub fn generate_from_manifest(path: &Path) -> Result<Self, LockfileError> {
        let graph =
            super::dependencies::resolve(path).map_err(|error| LockfileError(error.to_string()))?;
        let packages = graph
            .packages
            .into_iter()
            .map(|package| LockedPackage {
                name: package.name,
                version: package.version,
                source: Some(if package.path.is_some() { "path" } else { "registry" }.to_owned()),
                path: package.path,
                checksum: package.checksum,
            })
            .collect();
        Ok(Self::from_packages(packages))
    }

    pub fn validate_against_manifest(&self, path: &Path) -> Result<(), LockfileError> {
        let expected = Self::generate_from_manifest(path)?;
        if self.packages != expected.packages {
            return Err(LockfileError(
                "LockfileOutOfDate: Actus.lock does not match Actus.toml dependencies".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn sync(manifest_path: &Path) -> Result<PathBuf, LockfileError> {
        let lock_path = manifest_path.with_file_name("Actus.lock");
        let lockfile = Self::generate_from_manifest(manifest_path)?;
        let serialized = lockfile.serialize()?;
        std::fs::write(&lock_path, serialized).map_err(|error| {
            LockfileError(format!("cannot write `{}`: {error}", lock_path.display()))
        })?;
        Ok(lock_path)
    }

    fn validate_shape(&self) -> Result<(), LockfileError> {
        if self.lockfile_version != LOCKFILE_VERSION {
            return Err(LockfileError(format!(
                "unsupported Actus.lock version {}; expected {}",
                self.lockfile_version, LOCKFILE_VERSION
            )));
        }
        if self
            .packages
            .windows(2)
            .any(|packages| package_key(&packages[0]) > package_key(&packages[1]))
        {
            return Err(LockfileError(
                "Actus.lock packages must be sorted deterministically".to_owned(),
            ));
        }
        Ok(())
    }
}

fn package_key(package: &LockedPackage) -> (&str, &str, Option<&str>, Option<&str>) {
    (&package.name, &package.version, package.source.as_deref(), package.path.as_deref())
}

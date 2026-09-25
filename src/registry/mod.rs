mod cache;
mod index;
mod trust;

use std::fs;
use std::path::{Path, PathBuf};

pub use cache::PackageCache;
pub use index::{PackageIndex, PackageRecord};
pub use trust::TrustPolicy;

#[derive(Debug)]
pub struct RegistryError(pub String);

impl std::fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RegistryError {}

#[derive(Clone, Debug)]
pub struct LocalRegistry {
    root: PathBuf,
    index_path: PathBuf,
    package_directory: PathBuf,
}

impl LocalRegistry {
    pub fn open(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self { index_path: root.join("index.toml"), package_directory: root.join("packages"), root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn publish(
        &self,
        archive: &Path,
        record: PackageRecord,
        trust: &TrustPolicy,
    ) -> Result<PathBuf, RegistryError> {
        trust.verify_archive(archive, &record.checksum)?;
        let mut index = PackageIndex::load(&self.index_path)?;
        index.insert(record.clone())?;
        fs::create_dir_all(&self.package_directory)
            .map_err(|error| RegistryError(format!("cannot create registry packages: {error}")))?;
        let destination = self.package_directory.join(&record.archive);
        if !destination.is_file() {
            fs::copy(archive, &destination).map_err(|error| {
                RegistryError(format!("cannot write registry archive: {error}"))
            })?;
        }
        index.save(&self.index_path)?;
        Ok(destination)
    }
}

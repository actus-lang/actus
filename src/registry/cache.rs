use std::fs;
use std::path::{Path, PathBuf};

use crate::packaging::verify_archive_checksum;

use super::RegistryError;

#[derive(Clone, Debug)]
pub struct PackageCache {
    root: PathBuf,
}

impl PackageCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn archive_path(&self, checksum: &str) -> PathBuf {
        self.root.join(format!("{checksum}.arca"))
    }

    pub fn store(&self, archive: &Path, checksum: &str) -> Result<PathBuf, RegistryError> {
        verify_archive_checksum(archive, checksum)
            .map_err(|error| RegistryError(format!("cannot cache untrusted archive: {error}")))?;
        fs::create_dir_all(&self.root)
            .map_err(|error| RegistryError(format!("cannot create package cache: {error}")))?;
        let destination = self.archive_path(checksum);
        if !destination.is_file() {
            fs::copy(archive, &destination)
                .map_err(|error| RegistryError(format!("cannot cache package: {error}")))?;
        }
        Ok(destination)
    }
}

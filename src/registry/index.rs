use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::RegistryError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackageRecord {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub archive: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackageIndex {
    #[serde(rename = "package", default)]
    pub packages: Vec<PackageRecord>,
}

impl PackageIndex {
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let source = fs::read_to_string(path)
            .map_err(|error| RegistryError(format!("cannot read package index: {error}")))?;
        toml::from_str(&source)
            .map_err(|error| RegistryError(format!("cannot parse package index: {error}")))
    }

    pub fn save(&mut self, path: &Path) -> Result<(), RegistryError> {
        self.packages
            .sort_by(|left, right| (&left.name, &left.version).cmp(&(&right.name, &right.version)));
        let source = toml::to_string_pretty(self)
            .map_err(|error| RegistryError(format!("cannot serialize package index: {error}")))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                RegistryError(format!("cannot create index directory: {error}"))
            })?;
        }
        fs::write(path, source)
            .map_err(|error| RegistryError(format!("cannot write package index: {error}")))
    }

    pub fn insert(&mut self, record: PackageRecord) -> Result<(), RegistryError> {
        if let Some(existing) = self
            .packages
            .iter()
            .find(|entry| entry.name == record.name && entry.version == record.version)
        {
            if existing.checksum != record.checksum {
                return Err(RegistryError(format!(
                    "PackageConflict: {} {} already has a different checksum",
                    record.name, record.version
                )));
            }
            return Ok(());
        }
        self.packages.push(record);
        Ok(())
    }
}

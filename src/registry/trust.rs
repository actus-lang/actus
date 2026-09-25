use std::collections::BTreeSet;
use std::path::Path;

use crate::packaging::verify_archive_checksum;

use super::RegistryError;

#[derive(Clone, Debug, Default)]
pub struct TrustPolicy {
    trusted_checksums: BTreeSet<String>,
    require_trusted_checksum: bool,
}

impl TrustPolicy {
    pub fn integrity_only() -> Self {
        Self::default()
    }

    pub fn trusted_checksums(checksums: impl IntoIterator<Item = String>) -> Self {
        Self { trusted_checksums: checksums.into_iter().collect(), require_trusted_checksum: true }
    }

    pub fn verify_archive(&self, archive: &Path, checksum: &str) -> Result<(), RegistryError> {
        if checksum.is_empty() {
            return Err(RegistryError("TrustPolicy: package checksum is required".to_owned()));
        }
        verify_archive_checksum(archive, checksum)
            .map_err(|error| RegistryError(format!("TrustPolicy: {error}")))?;
        if self.require_trusted_checksum && !self.trusted_checksums.contains(checksum) {
            return Err(RegistryError(format!(
                "TrustPolicy: checksum `{checksum}` is not trusted"
            )));
        }
        Ok(())
    }
}

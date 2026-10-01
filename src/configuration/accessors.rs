use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::types::CompilerConfiguration;
use super::{BuildProfile, RuntimeProfile, SourceLimitMode};
use crate::target::{EntryContract, LinkerFlavor, TargetSpec};

impl CompilerConfiguration {
    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn has_manifest(&self) -> bool {
        self.project_root.join(super::manifest::MANIFEST_FILE_NAME).is_file()
    }

    pub fn default_entry_path(&self) -> PathBuf {
        self.source_root.join("main.act")
    }

    pub fn dependency_roots(&self) -> &std::collections::BTreeMap<String, PathBuf> {
        &self.dependency_roots
    }

    pub fn linker(&self) -> &OsStr {
        &self.linker
    }

    pub const fn linker_flavor(&self) -> LinkerFlavor {
        self.linker_flavor
    }

    pub const fn entry_contract(&self) -> EntryContract {
        self.entry_contract
    }

    pub const fn runtime_profile(&self) -> RuntimeProfile {
        self.runtime
    }

    pub const fn host_runtime_enabled(&self) -> bool {
        matches!(self.entry_contract, EntryContract::Hosted)
    }

    pub fn run_artifact_prefix(&self) -> &str {
        &self.run_artifact_prefix
    }

    pub fn native_backend(&self) -> &super::NativeBackendConfiguration {
        &self.native_backend
    }

    pub const fn source_limit_mode(&self) -> SourceLimitMode {
        self.source_limits
    }

    pub fn entry_symbol(&self) -> Option<&str> {
        self.entry_symbol.as_deref()
    }

    pub fn libraries(&self) -> &[super::LinkLibrary] {
        &self.libraries
    }

    pub fn library_paths(&self) -> &[PathBuf] {
        &self.library_paths
    }

    pub fn target(&self) -> &TargetSpec {
        &self.target
    }

    pub fn target_spec_hash(&self) -> &str {
        &self.target_spec_hash
    }

    pub const fn profile(&self) -> BuildProfile {
        self.profile
    }

    pub fn with_profile(mut self, profile: BuildProfile) -> Self {
        self.profile = profile;
        self.native_backend = self
            .native_backend
            .with_optimization_level(super::resolution::optimization_level(profile, self.profiles));
        self
    }

    pub fn capsula_target_directory(&self) -> PathBuf {
        self.project_root
            .join("capsula")
            .join(self.profile.directory_name())
            .join(self.target.triple().to_string())
    }
}

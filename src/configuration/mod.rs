mod accessors;
mod dependencies;
mod loading;
mod lockfile;
mod manifest;
mod resolution;
mod strict;
mod types;
mod version;

pub use crate::target::{EntryContract, LinkerFlavor};
pub use loading::package_identity;
pub use lockfile::{ActusLock, LockedPackage, LockfileError};
pub use manifest::OptimizationLevel;
pub use manifest::{BuildProfile, LibraryKind};
pub use strict::StrictConfigurationError;
pub use types::{
    CompilerConfiguration, ConfigurationError, LinkLibrary, NativeBackendConfiguration,
    PackageIdentity,
};
pub use version::{Version, VersionConstraint, VersionError};

pub(crate) use loading::manifest_path_in_directory;

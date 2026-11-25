//! Why a module path does not resolve to source.

use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug)]
pub enum ModuleResolutionError {
    InvalidPath(String),
    /// A `std::` module was imported by a build without the standard library.
    StandardLibraryUnavailable(String),
    MissingFacade {
        module: String,
        expected: PathBuf,
    },
    /// A builtin module exists but is unavailable for the configured target.
    IncompatibleRuntime {
        module: String,
        expected: PathBuf,
    },
    AmbiguousModule {
        module: String,
        directory: PathBuf,
        file: PathBuf,
    },
    BypassesFacade {
        module: String,
        parent: String,
        facade: PathBuf,
    },
    Io {
        path: PathBuf,
        message: String,
    },
    /// A test directory breaks the layout rules of ADR-0121; `path` is the
    /// directory or file that breaks them.
    TestLayout {
        path: PathBuf,
        message: String,
    },
}

impl Display for ModuleResolutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath(path) => write!(formatter, "invalid module path `{path}`"),
            Self::StandardLibraryUnavailable(module) => write!(
                formatter,
                "module `{module}` is part of the standard library, which this build does not \
                 use; set `runtime = \"std\"` under `[build]` in Actus.toml"
            ),
            Self::MissingFacade { module, expected } => write!(
                formatter,
                "module `{module}` is missing its canonical facade `{}`",
                expected.display()
            ),
            Self::IncompatibleRuntime { module, expected } => write!(
                formatter,
                "builtin module `{module}` is incompatible with the configured target; facade `{}` is unavailable",
                expected.display()
            ),
            Self::AmbiguousModule { module, directory, file } => write!(
                formatter,
                "module `{module}` has both directory `{}` and file `{}` roots",
                directory.display(),
                file.display()
            ),
            Self::BypassesFacade { module, parent, facade } => write!(
                formatter,
                "module `{module}` bypasses parent facade `{}` for `{parent}`",
                facade.display()
            ),
            Self::TestLayout { path, message } => {
                write!(formatter, "invalid test layout at `{}`: {message}", path.display())
            }
            Self::Io { path, message } => {
                write!(formatter, "cannot inspect module path `{}`: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for ModuleResolutionError {}

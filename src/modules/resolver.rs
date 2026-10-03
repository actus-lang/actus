use std::collections::{BTreeMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ModuleResolutionError {
    InvalidPath(String),
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
}

impl Display for ModuleResolutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath(path) => write!(formatter, "invalid module path `{path}`"),
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
            Self::Io { path, message } => {
                write!(formatter, "cannot inspect module path `{}`: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for ModuleResolutionError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedModule {
    module_path: String,
    facade: PathBuf,
    siblings: Vec<PathBuf>,
    children: Vec<ResolvedChildModule>,
}

impl ResolvedModule {
    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    pub fn facade(&self) -> &Path {
        &self.facade
    }

    pub fn siblings(&self) -> &[PathBuf] {
        &self.siblings
    }

    /// Returns child directory modules discovered beneath this module.
    pub fn children(&self) -> &[ResolvedChildModule] {
        &self.children
    }

    pub fn source_files(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.facade.as_path()).chain(self.siblings.iter().map(PathBuf::as_path))
    }
}

/// A child directory module and its canonical facade discovered by the resolver.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedChildModule {
    module_path: String,
    directory: PathBuf,
    facade: PathBuf,
}

impl ResolvedChildModule {
    /// Returns the canonical `::`-separated module path.
    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    /// Returns the child module directory.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Returns the child module's canonical facade.
    pub fn facade(&self) -> &Path {
        &self.facade
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleResolver {
    source_root: PathBuf,
    dependency_roots: BTreeMap<String, PathBuf>,
    runtime_source_root: Option<PathBuf>,
    runtime_module_roots: BTreeMap<String, PathBuf>,
}

impl ModuleResolver {
    pub fn new(source_root: impl Into<PathBuf>) -> Self {
        Self {
            source_root: source_root.into(),
            dependency_roots: BTreeMap::new(),
            runtime_source_root: None,
            runtime_module_roots: BTreeMap::new(),
        }
    }

    pub fn with_dependencies(
        source_root: impl Into<PathBuf>,
        dependency_roots: &BTreeMap<String, PathBuf>,
    ) -> Self {
        Self {
            source_root: source_root.into(),
            dependency_roots: dependency_roots.clone(),
            runtime_source_root: None,
            runtime_module_roots: BTreeMap::new(),
        }
    }

    pub fn with_dependencies_and_runtime(
        source_root: impl Into<PathBuf>,
        dependency_roots: &BTreeMap<String, PathBuf>,
        runtime_source_root: Option<&Path>,
        runtime_module_roots: &BTreeMap<String, PathBuf>,
    ) -> Self {
        Self {
            source_root: source_root.into(),
            dependency_roots: dependency_roots.clone(),
            runtime_source_root: runtime_source_root.map(Path::to_owned),
            runtime_module_roots: runtime_module_roots.clone(),
        }
    }

    pub fn resolve(&self, module_path: &str) -> Result<ResolvedModule, ModuleResolutionError> {
        let segments = parse_segments(module_path)?;
        if let Some(module_name) = segments.get(1)
            && segments.first() == Some(&"std")
        {
            let Some(root) = self.runtime_module_roots.get(*module_name) else {
                let root = self
                    .runtime_source_root
                    .as_ref()
                    .ok_or_else(|| ModuleResolutionError::InvalidPath(module_path.to_owned()))?;
                let expected = root.join(module_name).join(format!("{module_name}.act"));
                if expected.is_file() {
                    return Err(ModuleResolutionError::IncompatibleRuntime {
                        module: module_path.to_owned(),
                        expected,
                    });
                }
                return Err(ModuleResolutionError::MissingFacade {
                    module: module_path.to_owned(),
                    expected,
                });
            };
            if segments.len() != 2 {
                return Err(ModuleResolutionError::InvalidPath(module_path.to_owned()));
            }
            return resolve_directory(module_path, root, module_name);
        }
        let (root, module_segments) = self.root_and_segments(&segments);
        if module_segments.is_empty() {
            return Err(ModuleResolutionError::InvalidPath(module_path.to_owned()));
        }
        reject_facade_bypass(&root, module_segments, module_path)?;
        let directory =
            module_segments.iter().fold(root.clone(), |path, segment| path.join(segment));
        let file = self.module_file_root(&root, module_segments).with_extension("act");
        if directory.is_dir() && file.is_file() {
            return Err(ModuleResolutionError::AmbiguousModule {
                module: module_path.to_owned(),
                directory,
                file,
            });
        }
        if directory.is_dir() {
            return resolve_directory(module_path, &directory, segments.last().expect("path"));
        }
        if file.is_file() {
            return Ok(ResolvedModule {
                module_path: module_path.to_owned(),
                facade: file,
                siblings: Vec::new(),
                children: Vec::new(),
            });
        }
        Err(ModuleResolutionError::MissingFacade {
            module: module_path.to_owned(),
            expected: directory.join(format!("{}.act", module_segments.last().expect("path"))),
        })
    }

    fn root_and_segments<'a>(&'a self, segments: &'a [&str]) -> (PathBuf, &'a [&'a str]) {
        if segments.first() == Some(&"std")
            && let Some(root) = &self.runtime_source_root
        {
            return (root.clone(), &segments[1..]);
        }
        let root = self
            .dependency_roots
            .get(segments[0])
            .cloned()
            .unwrap_or_else(|| self.source_root.clone());
        (root, segments)
    }

    fn module_file_root(&self, root: &Path, segments: &[&str]) -> PathBuf {
        segments.iter().fold(root.to_owned(), |path, segment| path.join(segment))
    }
}

fn parse_segments(module_path: &str) -> Result<Vec<&str>, ModuleResolutionError> {
    let segments = module_path.split("::").collect::<Vec<_>>();
    if segments.is_empty() || segments.iter().any(|segment| !valid_segment(segment)) {
        return Err(ModuleResolutionError::InvalidPath(module_path.to_owned()));
    }
    Ok(segments)
}

fn valid_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    let Some(first) = characters.next() else { return false };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn reject_facade_bypass(
    root: &Path,
    segments: &[&str],
    module_path: &str,
) -> Result<(), ModuleResolutionError> {
    if segments.len() < 2 {
        return Ok(());
    }
    for parent_length in 1..segments.len() {
        let parent = segments[..parent_length]
            .iter()
            .fold(root.to_owned(), |path, segment| path.join(segment));
        if !parent.is_dir() {
            continue;
        }
        let parent_name = segments[parent_length - 1];
        let facade = parent.join(format!("{parent_name}.act"));
        if facade.is_file() {
            return Err(ModuleResolutionError::BypassesFacade {
                module: module_path.to_owned(),
                parent: segments[..parent_length].join("::"),
                facade,
            });
        }
    }
    Ok(())
}

fn resolve_directory(
    module_path: &str,
    directory: &Path,
    module_name: &str,
) -> Result<ResolvedModule, ModuleResolutionError> {
    let facade = directory.join(format!("{module_name}.act"));
    if !facade.is_file() {
        return Err(ModuleResolutionError::MissingFacade {
            module: module_path.to_owned(),
            expected: facade,
        });
    }
    let mut siblings = discover_siblings(directory, &facade)?;
    siblings.sort_by_key(|path| normalized_path(path));
    let children = discover_children(module_path, directory, &siblings)?;
    Ok(ResolvedModule { module_path: module_path.to_owned(), facade, siblings, children })
}

fn discover_siblings(
    directory: &Path,
    facade: &Path,
) -> Result<Vec<PathBuf>, ModuleResolutionError> {
    let entries = fs::read_dir(directory).map_err(|error| io_error(directory, error))?;
    let mut siblings = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| io_error(directory, error))?.path();
        if is_act_file(&path) && path != facade {
            siblings.push(path);
        }
    }
    Ok(siblings)
}

fn discover_children(
    module_path: &str,
    directory: &Path,
    siblings: &[PathBuf],
) -> Result<Vec<ResolvedChildModule>, ModuleResolutionError> {
    let sibling_names = siblings
        .iter()
        .filter_map(|path| path.file_stem().and_then(|stem| stem.to_str()))
        .collect::<HashSet<_>>();
    let entries = fs::read_dir(directory).map_err(|error| io_error(directory, error))?;
    let mut children = Vec::new();
    for entry in entries {
        let child_directory = entry.map_err(|error| io_error(directory, error))?.path();
        if !child_directory.is_dir() {
            continue;
        }
        let Some(child_name) =
            child_directory.file_name().and_then(|name| name.to_str()).map(str::to_owned)
        else {
            continue;
        };
        if !valid_segment(&child_name) {
            continue;
        }
        let child_facade = child_directory.join(format!("{child_name}.act"));
        if sibling_names.contains(child_name.as_str()) {
            return Err(ModuleResolutionError::AmbiguousModule {
                module: format!("{module_path}::{child_name}"),
                directory: child_directory,
                file: directory.join(format!("{child_name}.act")),
            });
        }
        if !child_facade.is_file() {
            return Err(ModuleResolutionError::MissingFacade {
                module: format!("{module_path}::{child_name}"),
                expected: child_facade,
            });
        }
        children.push(ResolvedChildModule {
            module_path: format!("{module_path}::{child_name}"),
            directory: child_directory,
            facade: child_facade,
        });
    }
    children.sort_by(|left, right| left.module_path.cmp(&right.module_path));
    Ok(children)
}

fn is_act_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some("act")
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn io_error(path: &Path, error: std::io::Error) -> ModuleResolutionError {
    ModuleResolutionError::Io { path: path.to_owned(), message: error.to_string() }
}

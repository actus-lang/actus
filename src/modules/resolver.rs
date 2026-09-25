use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ModuleResolutionError {
    InvalidPath(String),
    MissingFacade { module: String, expected: PathBuf },
    AmbiguousModule { module: String, directory: PathBuf, file: PathBuf },
    Io { path: PathBuf, message: String },
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
            Self::AmbiguousModule { module, directory, file } => write!(
                formatter,
                "module `{module}` has both directory `{}` and file `{}` roots",
                directory.display(),
                file.display()
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

    pub fn source_files(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.facade.as_path()).chain(self.siblings.iter().map(PathBuf::as_path))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleResolver {
    source_root: PathBuf,
    dependency_roots: BTreeMap<String, PathBuf>,
}

impl ModuleResolver {
    pub fn new(source_root: impl Into<PathBuf>) -> Self {
        Self { source_root: source_root.into(), dependency_roots: BTreeMap::new() }
    }

    pub fn with_dependencies(
        source_root: impl Into<PathBuf>,
        dependency_roots: &BTreeMap<String, PathBuf>,
    ) -> Self {
        Self { source_root: source_root.into(), dependency_roots: dependency_roots.clone() }
    }

    pub fn resolve(&self, module_path: &str) -> Result<ResolvedModule, ModuleResolutionError> {
        let segments = parse_segments(module_path)?;
        let root = self
            .dependency_roots
            .get(segments[0])
            .cloned()
            .unwrap_or_else(|| self.source_root.clone());
        let directory = segments.iter().fold(root.clone(), |path, segment| path.join(segment));
        let file = self.module_file_root(&root, &segments).with_extension("act");
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
            });
        }
        Err(ModuleResolutionError::MissingFacade {
            module: module_path.to_owned(),
            expected: directory.join(format!("{}.act", segments.last().expect("path"))),
        })
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
    Ok(ResolvedModule { module_path: module_path.to_owned(), facade, siblings })
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

fn is_act_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some("act")
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn io_error(path: &Path, error: std::io::Error) -> ModuleResolutionError {
    ModuleResolutionError::Io { path: path.to_owned(), message: error.to_string() }
}

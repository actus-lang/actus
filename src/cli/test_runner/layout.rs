//! Discovery and validation of the test layout (ADR-0121).
//!
//! Tests live in two places: a `tests` directory next to a module facade, and
//! `tests/` at the project root. Everything else that looks like a test is a
//! layout error that names the place the test belongs.

use std::fs;
use std::path::{Path, PathBuf};

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{module_diagnostic, render_diagnostic};
use crate::lexer::TokenKind;
use crate::modules::{ModuleError, ModuleResolutionError, reject_test_verbs};
use crate::session::CompilationSession;

use super::super::conformance::validate_source_limits;
use super::TestOptions;
use super::collection::localized_test_diagnostic;

/// What a test directory tests.
pub(super) enum TestScope {
    /// The `tests` directory of one module, compiled as part of that module.
    Module(String),
    /// The project `tests/` directory, compiled against what modules open.
    Project,
}

/// One `tests` directory and its `.act` files. All files of a directory are
/// compiled together as one namespace.
pub(super) struct TestDirectory {
    pub(super) scope: TestScope,
    pub(super) files: Vec<PathBuf>,
}

/// Finds every test directory of the project and rejects every file that is
/// in the wrong place. Production source files are read once: strict source
/// limits are validated and a `meta test` verb in one is an error.
pub(super) fn discover_test_directories(
    configuration: &CompilerConfiguration,
    options: &TestOptions,
    session: &CompilationSession,
) -> Result<Vec<TestDirectory>, String> {
    let source_root = configuration.source_root();
    let project_tests = configuration.project_root().join("tests");
    let mut directories = Vec::new();
    let mut pending = vec![source_root.to_owned()];
    while let Some(directory) = pending.pop() {
        for path in sorted_entries(&directory) {
            if path.is_dir() {
                if path == project_tests || is_data_directory(&path) {
                    continue;
                }
                if path.file_name().is_some_and(|name| name == "tests") {
                    directories.extend(module_test_directory(&path, source_root)?);
                } else {
                    pending.push(path);
                }
            } else if is_act_file(&path) {
                validate_production_source(&path, configuration, options, session)?;
            }
        }
    }
    directories.extend(project_test_directory(&project_tests, source_root)?);
    directories.sort_by(|left, right| left.files.cmp(&right.files));
    Ok(directories)
}

fn module_test_directory(
    directory: &Path,
    source_root: &Path,
) -> Result<Option<TestDirectory>, String> {
    let parent = directory.parent().unwrap_or(source_root);
    if parent == source_root {
        return Err(layout_error(
            directory,
            "the root unit has no tests directory; move its logic into a module and test it \
             there, or test it from `tests/` through what it opens",
        ));
    }
    let Some(module) = module_path_of_directory(parent, source_root) else {
        return Err(layout_error(
            directory,
            &format!(
                "a tests directory belongs to a module, but `{}` has no facade `{}.act`",
                parent.display(),
                parent.file_name().and_then(|name| name.to_str()).unwrap_or_default()
            ),
        ));
    };
    if directory.join("tests.act").is_file() {
        return Err(layout_error(
            directory,
            "a `tests` directory holds the tests of its module and is never a module; remove \
             its facade `tests.act`",
        ));
    }
    let files = flat_act_files(directory)?;
    Ok((!files.is_empty()).then_some(TestDirectory { scope: TestScope::Module(module), files }))
}

fn project_test_directory(
    directory: &Path,
    source_root: &Path,
) -> Result<Option<TestDirectory>, String> {
    let files = flat_act_files_with(directory, |nested| {
        // The old mirror layout: name the module tests directory the file
        // belongs in, computed the way the mirror target was.
        let relative = nested.strip_prefix(directory).unwrap_or(nested);
        let name = nested.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        let destination = mirror_module(relative, source_root).map_or_else(
            || format!("no module mirrors it; project tests live directly in `tests/`: move it to `tests/{name}`"),
            |module_directory| {
                format!("move it to `{}`", module_directory.join("tests").join(name).display())
            },
        );
        layout_error(
            nested,
            &format!(
                "project tests are directly in `tests/`, not in subdirectories; {destination}"
            ),
        )
    })?;
    Ok((!files.is_empty()).then_some(TestDirectory { scope: TestScope::Project, files }))
}

/// The `.act` files directly in a module tests directory; a file in a
/// subdirectory is an error because one directory is one namespace.
fn flat_act_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    flat_act_files_with(directory, |nested| {
        layout_error(
            nested,
            &format!(
                "a tests directory holds `.act` files only, with no subdirectories; move it \
                 into `{}`",
                directory.display()
            ),
        )
    })
}

fn flat_act_files_with(
    directory: &Path,
    nested_error: impl Fn(&Path) -> String,
) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for path in sorted_entries(directory) {
        if path.is_dir() {
            if is_data_directory(&path) {
                continue;
            }
            if let Some(nested) = first_act_file(&path) {
                return Err(nested_error(&nested));
            }
        } else if is_act_file(&path) {
            files.push(path);
        }
    }
    Ok(files)
}

fn first_act_file(directory: &Path) -> Option<PathBuf> {
    sorted_entries(directory).into_iter().find_map(|path| {
        if path.is_dir() {
            (!is_data_directory(&path)).then(|| first_act_file(&path)).flatten()
        } else {
            is_act_file(&path).then_some(path)
        }
    })
}

/// The module directory a file of the old `tests/<path>.act` mirror layout
/// belonged to: `src/<path>.act`, or the canonical facade `src/<path>/<stem>.act`.
fn mirror_module(relative: &Path, source_root: &Path) -> Option<PathBuf> {
    let candidate = source_root.join(relative);
    let source = if candidate.is_file() {
        candidate
    } else {
        let stem = candidate.file_stem()?.to_str()?;
        let facade = candidate.with_file_name(stem).join(format!("{stem}.act"));
        facade.is_file().then_some(facade)?
    };
    let mut directory = source.parent()?.to_owned();
    // The owning module is the nearest ancestor directory with its facade.
    while directory.starts_with(source_root) && directory != source_root {
        let name = directory.file_name()?.to_str()?;
        if directory.join(format!("{name}.act")).is_file() {
            return Some(directory);
        }
        directory = directory.parent()?.to_owned();
    }
    None
}

/// The module path of a module directory, when it and every ancestor below
/// the source root have their facade.
fn module_path_of_directory(directory: &Path, source_root: &Path) -> Option<String> {
    let relative = directory.strip_prefix(source_root).ok()?;
    let mut current = source_root.to_owned();
    let mut segments = Vec::new();
    for component in relative.components() {
        let name = component.as_os_str().to_str()?;
        current.push(name);
        if !current.join(format!("{name}.act")).is_file() {
            return None;
        }
        segments.push(name.to_owned());
    }
    (!segments.is_empty()).then(|| segments.join("::"))
}

fn validate_production_source(
    path: &Path,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
    session: &CompilationSession,
) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
    if !validate_source_limits(path, &source, options.mode, configuration) {
        return Err(format!("strict source-limit validation failed for `{}`", path.display()));
    }
    let parsed = session.parse(path, &source);
    if !parsed.tokens().iter().any(|token| matches!(token.kind, TokenKind::Meta)) {
        return Ok(());
    }
    // A file that does not parse is reported when its module is loaded.
    let Ok(program) = parsed.program() else { return Ok(()) };
    let module_directory = owning_module_directory(path, configuration.source_root());
    let tests_directory = module_directory.map(|directory| directory.join("tests"));
    reject_test_verbs(path, &program, tests_directory.as_deref()).map_err(|error| {
        localized_test_diagnostic(path, &source, session.source_map(), module_diagnostic(&error))
    })
}

/// The directory of the module that owns a production file, or `None` for the
/// root unit.
fn owning_module_directory(path: &Path, source_root: &Path) -> Option<PathBuf> {
    let mut directory = path.parent()?.to_owned();
    while directory.starts_with(source_root) && directory != source_root {
        let name = directory.file_name()?.to_str()?;
        if directory.join(format!("{name}.act")).is_file() {
            return Some(directory);
        }
        directory = directory.parent()?.to_owned();
    }
    None
}

fn layout_error(path: &Path, message: &str) -> String {
    let error = ModuleError::Resolution(ModuleResolutionError::TestLayout {
        path: path.to_owned(),
        message: message.to_owned(),
    });
    format!("{}: {}", path.display(), render_diagnostic("", &module_diagnostic(&error)))
}

fn sorted_entries(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else { return Vec::new() };
    let mut paths = entries.flatten().map(|entry| entry.path()).collect::<Vec<_>>();
    paths.sort();
    paths
}

fn is_act_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "act")
}

/// `fixtures` and `snapshots` hold data for tests, not tests.
fn is_data_directory(path: &Path) -> bool {
    path.file_name().is_some_and(|name| matches!(name.to_str(), Some("fixtures" | "snapshots")))
}

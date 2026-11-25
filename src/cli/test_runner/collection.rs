use std::fs;
use std::path::{Path, PathBuf};

use crate::SharedSourceMap;
use crate::ast::{ImportDecl, MetaAttribute, Program, TestFault, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{module_diagnostic, render_diagnostic, semantic_diagnostic};
use crate::modules::{
    ModuleCompilationPlan, ModuleError, ModuleResolver, build_compilation_plan,
    check_unique_declarations, load_module_unit, validate_plan_scopes,
};
use crate::semantic::filter_program_for_target_checked;
use crate::session::{CompilationSession, FrontendError};

use super::super::conformance::{ConformanceMode, validate_source_limits};
use super::TestOptions;
use super::layout::{TestDirectory, TestScope};

/// One selected test. What it is compiled with is shared by its group.
pub(super) struct DiscoveredTest {
    pub(super) path: PathBuf,
    pub(super) module: String,
    pub(super) display_module: String,
    pub(super) relative_path: PathBuf,
    pub(super) name: String,
    pub(super) fault: Option<TestFault>,
}

/// The selected tests of one test directory and the one program they are all
/// compiled from (ADR-0121): the directory's namespace after target
/// filtering, still holding every test verb of the directory.
pub(super) struct TestGroup {
    pub(super) label: String,
    pub(super) source_map: SharedSourceMap,
    pub(super) program: Program,
    pub(super) compilation_plan: ModuleCompilationPlan,
    /// The selected tests in program order. The position of a test in this
    /// list is the index its executable is told to run.
    pub(super) tests: Vec<DiscoveredTest>,
}

pub(super) struct TestCollection {
    pub(super) groups: Vec<TestGroup>,
    /// Every selected test as `(group, position in group)`, in canonical
    /// order: by file path, then by name.
    pub(super) order: Vec<(usize, usize)>,
    pub(super) filtered: usize,
}

impl TestCollection {
    pub(super) fn test(&self, (group, position): (usize, usize)) -> &DiscoveredTest {
        &self.groups[group].tests[position]
    }
}

struct CollectedDirectory {
    group: Option<TestGroup>,
    filtered: usize,
}

pub(super) fn collect_tests(
    directories: Vec<TestDirectory>,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
    session: &CompilationSession,
) -> Result<TestCollection, String> {
    if directories.is_empty() {
        return Ok(TestCollection { groups: Vec::new(), order: Vec::new(), filtered: 0 });
    }
    let worker_count =
        std::thread::available_parallelism().map(usize::from).unwrap_or(1).min(directories.len());
    let chunk_size = directories.len().div_ceil(worker_count);
    // One session for the complete test run (ADR-0109): every worker shares
    // source provenance and memoized parse results.
    let results = std::thread::scope(|scope| {
        directories
            .chunks(chunk_size)
            .map(|chunk| {
                let session = session.clone();
                scope.spawn(move || collect_chunk(chunk, configuration, options, &session))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("test collection worker panicked"))
            .collect::<Result<Vec<_>, _>>()
    })?;
    let mut groups = Vec::new();
    let mut filtered = 0;
    for collected in results.into_iter().flatten() {
        groups.extend(collected.group);
        filtered += collected.filtered;
    }
    let mut order = (0..groups.len())
        .flat_map(|group| (0..groups[group].tests.len()).map(move |position| (group, position)))
        .collect::<Vec<_>>();
    order.sort_by(|left, right| {
        let (left, right) = (&groups[left.0].tests[left.1], &groups[right.0].tests[right.1]);
        left.path.cmp(&right.path).then(left.name.cmp(&right.name))
    });
    Ok(TestCollection { groups, order, filtered })
}

/// One collection worker: the directories of one chunk, in order.
fn collect_chunk(
    chunk: &[TestDirectory],
    configuration: &CompilerConfiguration,
    options: &TestOptions,
    session: &CompilationSession,
) -> Result<Vec<CollectedDirectory>, String> {
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_session(session.clone());
    chunk
        .iter()
        .map(|directory| {
            collect_directory_tests(directory, configuration, options, &resolver, session)
        })
        .collect()
}

/// One parsed file of a test directory.
struct TestFile {
    path: PathBuf,
    relative_path: PathBuf,
    program: Program,
}

/// Reads, validates and parses the files of one test directory.
fn parse_directory_files(
    directory: &TestDirectory,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
    session: &CompilationSession,
) -> Result<Vec<TestFile>, String> {
    let mut files = Vec::new();
    for path in &directory.files {
        let source = fs::read_to_string(path)
            .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
        validate_source(path, &source, options.mode, configuration)?;
        let program = session.parse(path, &source).program().map_err(|error| match error {
            FrontendError::Lex(_) => format!("cannot lex `{}`", path.display()),
            FrontendError::Parse(error) => parse_source_error(path, session.source_map(), error),
        })?;
        let relative_path =
            path.strip_prefix(configuration.project_root()).unwrap_or(path).to_path_buf();
        files.push(TestFile { path: path.clone(), relative_path, program });
    }
    Ok(files)
}

/// Compiles every file of one test directory together as one namespace and
/// returns the selected tests of its files.
fn collect_directory_tests(
    directory: &TestDirectory,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
    resolver: &ModuleResolver,
    session: &CompilationSession,
) -> Result<CollectedDirectory, String> {
    let source_map = session.source_map().clone();
    let files = parse_directory_files(directory, configuration, options, session)?;
    let label = match &directory.scope {
        TestScope::Module(module) => module.as_str(),
        TestScope::Project => "tests",
    };
    let selected = files.iter().any(|file| {
        test_names(&file.program)
            .iter()
            .any(|name| raw_test_matches(name, label, &file.relative_path, options))
    });
    if !selected {
        // Not compiled: its tests are counted from the parsed files alone, so
        // `filtered` is every discovered test that does not run. A file that
        // fails to read or parse still fails the run above, because its tests
        // cannot be counted.
        let filtered = files.iter().map(|file| test_count(&file.program)).sum();
        return Ok(CollectedDirectory { group: None, filtered });
    }
    let (program, plan) = compile_directory(directory, &files, resolver, &source_map)?;
    let declared = test_count(&program);
    let program =
        filter_program_for_target_checked(&program, configuration.target()).map_err(|error| {
            localized_test_diagnostic(&files[0].path, "", &source_map, semantic_diagnostic(&error))
        })?;
    let filtered = declared.saturating_sub(test_count(&program));
    if let Err(error) = crate::semantic::analyze_resolved(&program) {
        let diagnostic = semantic_diagnostic(&error);
        return Err(localized_test_diagnostic(&files[0].path, "", &source_map, diagnostic));
    }
    let discovered = discover_tests(label, &files, &program);
    let discovered_count = discovered.len();
    let tests =
        discovered.into_iter().filter(|test| test_matches(test, options)).collect::<Vec<_>>();
    let filtered = filtered + discovered_count - tests.len();
    let group = (!tests.is_empty()).then(|| TestGroup {
        label: label.to_owned(),
        source_map,
        program,
        compilation_plan: plan,
        tests,
    });
    Ok(CollectedDirectory { group, filtered })
}

/// Merges the files of a test directory into the program they are compiled
/// as. A module's test files are compiled as sibling files of the module, so
/// they see its private declarations and are held to the module's scope rules
/// (ADR-0110); a project's test files form one root program. In both cases
/// imports are checked as for any importer.
fn compile_directory(
    directory: &TestDirectory,
    files: &[TestFile],
    resolver: &ModuleResolver,
    source_map: &SharedSourceMap,
) -> Result<(Program, ModuleCompilationPlan), String> {
    let first = &files[0].path;
    let fail = |error: ModuleError| module_source_error(first, "", source_map, error);
    let program = match &directory.scope {
        TestScope::Module(module_path) => {
            let paths = files.iter().map(|file| file.path.clone()).collect();
            let resolver = resolver.clone().with_module_tests(module_path, paths);
            let unit = load_module_unit(&resolver, module_path)
                .map_err(|error| format!("cannot load module `{module_path}`: {}", fail(error)))?;
            validate_plan_scopes(std::slice::from_ref(&unit)).map_err(fail)?;
            let mut program = unit.implementation().clone();
            remove_internal_module_imports(&mut program, module_path);
            let plan = build_compilation_plan(&program, &resolver).map_err(fail)?;
            return Ok((plan.caller().clone(), plan));
        }
        TestScope::Project => {
            let named = files.iter().map(|file| (file.path.as_path(), &file.program));
            check_unique_declarations(&named.collect::<Vec<_>>()).map_err(fail)?;
            merge_project_programs(files)
        }
    };
    let plan = build_compilation_plan(&program, resolver).map_err(fail)?;
    Ok((plan.caller().clone(), plan))
}

/// The root program of the project tests: every declaration of every file.
/// Files of one directory share one namespace, so an import of the same module
/// in several files is one import, `open` when any of them is.
fn merge_project_programs(files: &[TestFile]) -> Program {
    let mut declarations: Vec<TopLevelDecl> = Vec::new();
    for declaration in files.iter().flat_map(|file| &file.program.declarations) {
        let TopLevelDecl::Import(import) = declaration else {
            declarations.push(declaration.clone());
            continue;
        };
        let binding = |import: &ImportDecl| {
            import.alias.clone().or_else(|| import.path.rsplit("::").next().map(str::to_owned))
        };
        let existing = declarations.iter_mut().find_map(|candidate| match candidate {
            TopLevelDecl::Import(other)
                if other.path == import.path && binding(other) == binding(import) =>
            {
                Some(other)
            }
            _ => None,
        });
        match existing {
            Some(other) => {
                other.is_open |= import.is_open;
                for name in &import.selected {
                    if !other.selected.contains(name) {
                        other.selected.push(name.clone());
                    }
                }
            }
            None => declarations.push(declaration.clone()),
        }
    }
    Program { file_metadata: Vec::new(), declarations }
}

pub(super) fn localized_test_diagnostic(
    fallback_path: &Path,
    fallback_source: &str,
    source_map: &SharedSourceMap,
    diagnostic: crate::diagnostics::Diagnostic,
) -> String {
    source_map
        .lock()
        .localize(&diagnostic)
        .map(|(path, source, diagnostic)| {
            format!("{}: {}", path.display(), render_diagnostic(&source, &diagnostic))
        })
        .unwrap_or_else(|| {
            format!(
                "{}: {}",
                fallback_path.display(),
                render_diagnostic(fallback_source, &diagnostic)
            )
        })
}

fn validate_source(
    path: &Path,
    source: &str,
    mode: ConformanceMode,
    configuration: &CompilerConfiguration,
) -> Result<(), String> {
    if validate_source_limits(path, source, mode, configuration) {
        Ok(())
    } else {
        Err(format!("strict source-limit validation failed for `{}`", path.display()))
    }
}

fn parse_source_error(
    path: &Path,
    source_map: &SharedSourceMap,
    error: crate::parser::ParseError,
) -> String {
    let diagnostic = crate::diagnostics::parse_diagnostic(&error);
    source_map.lock().localize(&diagnostic).map_or_else(
        || format!("cannot parse `{}`: {error:?}", path.display()),
        |(localized_path, source, diagnostic)| {
            format!("{}: {}", localized_path.display(), render_diagnostic(&source, &diagnostic))
        },
    )
}

fn module_source_error(
    path: &Path,
    source: &str,
    source_map: &SharedSourceMap,
    error: ModuleError,
) -> String {
    let diagnostic = module_diagnostic(&error);
    source_map.lock().localize(&diagnostic).map_or_else(
        || format!("{}: {}", path.display(), render_diagnostic(source, &diagnostic)),
        |(path, source, diagnostic)| {
            format!("{}: {}", path.display(), render_diagnostic(&source, &diagnostic))
        },
    )
}

fn remove_internal_module_imports(program: &mut Program, module_path: &str) {
    program.declarations.retain(|declaration| {
        let TopLevelDecl::Import(import) = declaration else { return true };
        !is_internal_module_path(&import.path, module_path)
    });
}

fn is_internal_module_path(import_path: &str, module_path: &str) -> bool {
    import_path == module_path || module_path.starts_with(&format!("{import_path}::"))
}

/// The tests of a directory: its files' `meta test` verbs, named
/// `<module path>::<verb>` or `tests::<verb>`.
fn discover_tests(label: &str, files: &[TestFile], program: &Program) -> Vec<DiscoveredTest> {
    let declared = files
        .iter()
        .flat_map(|file| test_names(&file.program).into_iter().map(move |name| (name, file)))
        .collect::<Vec<_>>();
    program
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Verb(verb) = declaration else { return None };
            if !verb.metadata.contains(&MetaAttribute::Test) {
                return None;
            }
            let (_, file) = declared.iter().find(|(name, _)| *name == verb.name)?;
            Some(DiscoveredTest {
                path: file.path.clone(),
                module: label.to_owned(),
                display_module: label.to_owned(),
                relative_path: file.relative_path.clone(),
                name: verb.name.clone(),
                fault: verb.metadata.iter().find_map(|attribute| match attribute {
                    MetaAttribute::Fault(fault) => Some(*fault),
                    _ => None,
                }),
            })
        })
        .collect()
}

fn test_matches(test: &DiscoveredTest, options: &TestOptions) -> bool {
    raw_test_matches(&test.name, &test.module, &test.relative_path, options)
}

fn raw_test_matches(name: &str, module: &str, relative_path: &Path, options: &TestOptions) -> bool {
    let identity = format!("{module}::{name}");
    let matches = |value: &str, candidate: &str| {
        if options.exact { candidate == value } else { candidate.contains(value) }
    };
    options
        .filter
        .as_deref()
        .is_none_or(|filter| matches(filter, &identity) || matches(filter, name))
        && options.module.as_deref().is_none_or(|filter| matches(filter, module))
        && options
            .file
            .as_deref()
            .is_none_or(|file| matches(file, &relative_path.to_string_lossy()))
}

fn test_names(program: &Program) -> Vec<String> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) if verb.metadata.contains(&MetaAttribute::Test) => {
                Some(verb.name.clone())
            }
            _ => None,
        })
        .collect()
}

fn test_count(program: &Program) -> usize {
    program
        .declarations
        .iter()
        .filter(|declaration| {
            matches!(declaration, TopLevelDecl::Verb(verb) if verb.metadata.contains(&MetaAttribute::Test))
        })
        .count()
}

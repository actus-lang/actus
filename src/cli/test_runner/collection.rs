use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{MetaAttribute, Program, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{module_diagnostic, render_diagnostic, semantic_diagnostic};
use crate::lexer::{TokenKind, scan};
use crate::modules::{
    ModuleCompilationPlan, ModuleResolver, build_test_compilation_plan, parse_module,
};
use crate::parser::parse;
use crate::semantic::filter_program_for_target;

use super::super::conformance::{ConformanceMode, validate_source_limits};
use super::TestOptions;

pub(super) struct DiscoveredTest {
    pub(super) path: PathBuf,
    pub(super) module: String,
    pub(super) relative_path: PathBuf,
    pub(super) name: String,
    pub(super) program: Program,
    pub(super) compilation_plan: Option<ModuleCompilationPlan>,
}

pub(super) struct TestCollection {
    pub(super) tests: Vec<DiscoveredTest>,
    pub(super) filtered: usize,
}

struct CollectedFile {
    tests: Vec<DiscoveredTest>,
    filtered: usize,
}

pub(super) fn collect_tests(
    files: Vec<PathBuf>,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
) -> Result<TestCollection, String> {
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    );
    let mut tests = Vec::new();
    let mut filtered = 0;
    for path in files {
        let collected = collect_file_tests(&path, configuration, options, &resolver)?;
        tests.extend(collected.tests);
        filtered += collected.filtered;
    }
    tests.sort_by(|left, right| left.path.cmp(&right.path).then(left.name.cmp(&right.name)));
    Ok(TestCollection { tests, filtered })
}

fn collect_file_tests(
    path: &Path,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
    resolver: &ModuleResolver,
) -> Result<CollectedFile, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
    if options.mirror && path.starts_with(configuration.project_root().join("tests")) {
        validate_test_mirror(path, configuration)?;
    }
    validate_source(path, &source, options.mode, configuration)?;
    let Some((_source_program, program, compilation_plan, test_names)) = parse_source(
        path,
        &source,
        resolver,
        configuration.source_root(),
        configuration.project_root(),
        options,
    )?
    else {
        return Ok(CollectedFile { tests: Vec::new(), filtered: 0 });
    };
    let declared = test_count(&program);
    let program = filter_program_for_target(&program, configuration.target());
    let filtered = declared.saturating_sub(test_count(&program));
    validate_semantics(path, &source, &program, options.mode)?;
    let module =
        module_path_for_test(path, configuration.source_root(), configuration.project_root());
    let relative_path =
        path.strip_prefix(configuration.project_root()).unwrap_or(path).to_path_buf();
    let discovered =
        discover_tests(path, &module, &relative_path, program, compilation_plan, &test_names);
    let selected = discovered.iter().filter(|test| test_matches(test, options)).count();
    let selection_filtered = discovered.len().saturating_sub(selected);
    Ok(CollectedFile {
        tests: discovered.into_iter().filter(|test| test_matches(test, options)).collect(),
        filtered: filtered + selection_filtered,
    })
}

fn validate_test_mirror(path: &Path, configuration: &CompilerConfiguration) -> Result<(), String> {
    let tests_root = configuration.project_root().join("tests");
    let relative = path
        .strip_prefix(&tests_root)
        .map_err(|_| format!("test file `{}` is outside the tests directory", path.display()))?;
    if !is_nested_test_path(relative) {
        return Ok(());
    }
    let candidate = configuration.source_root().join(relative);
    if candidate.is_file() {
        return Ok(());
    }
    let Some(stem) = candidate.file_stem().and_then(|value| value.to_str()) else {
        return Err(format!("test file `{}` has no source mirror", path.display()));
    };
    let facade = candidate.with_file_name(stem).join(format!("{stem}.act"));
    if facade.is_file() {
        Ok(())
    } else {
        Err(format!(
            "test file `{}` has no source mirror; expected `{}` or `{}`",
            path.display(),
            candidate.display(),
            facade.display()
        ))
    }
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

fn parse_source(
    path: &Path,
    source: &str,
    resolver: &ModuleResolver,
    source_root: &Path,
    project_root: &Path,
    options: &TestOptions,
) -> Result<Option<(Program, Program, Option<ModuleCompilationPlan>, Vec<String>)>, String> {
    let (tokens, errors) = scan(source);
    if !tokens.iter().any(|token| matches!(token.kind, TokenKind::Meta)) {
        return Ok(None);
    }
    if !errors.is_empty() {
        return Err(format!("cannot lex `{}`", path.display()));
    }
    let source_program =
        parse(tokens).map_err(|error| format!("cannot parse `{}`: {error:?}", path.display()))?;
    let test_names = test_names(&source_program);
    if test_names.is_empty() {
        return Ok(None);
    }
    let module_path = module_path_for_test(path, source_root, project_root);
    let module = if module_path.is_empty() { "<root>" } else { &module_path };
    let relative_path = path.strip_prefix(project_root).unwrap_or(path);
    if !test_names.iter().any(|name| raw_test_matches(name, module, relative_path, options)) {
        return Ok(None);
    }
    let compilation_source = (!module_path.is_empty())
        .then_some(module_path.as_str())
        .map(|module_path| {
            parse_module(resolver, module_path)
                .map_err(|error| format!("cannot load module `{module_path}`: {error}"))
        })
        .transpose()?
        .map(|mut program| {
            if !path.starts_with(source_root) {
                append_mirrored_test_declarations(&mut program, &source_program, &module_path);
            }
            remove_internal_module_imports(&mut program, &module_path);
            program
        })
        .unwrap_or_else(|| source_program.clone());
    let resolved = build_test_compilation_plan(&compilation_source, resolver)
        .map(|plan| (plan.caller().clone(), Some(plan)));
    resolved.map(|(program, plan)| Some((compilation_source, program, plan, test_names))).map_err(
        |error| {
            let diagnostic = module_diagnostic(&error).with_source_path(path.display().to_string());
            format!("{}: {}", path.display(), render_diagnostic(source, &diagnostic))
        },
    )
}

fn append_mirrored_test_declarations(
    module_program: &mut Program,
    test_program: &Program,
    module_path: &str,
) {
    module_program.declarations.extend(
        test_program
            .declarations
            .iter()
            .filter(|declaration| match declaration {
                TopLevelDecl::Import(import) => import.path != module_path,
                TopLevelDecl::OpenSibling(_) => false,
                _ => true,
            })
            .cloned(),
    );
}

fn remove_internal_module_imports(program: &mut Program, module_path: &str) {
    program.declarations.retain(|declaration| {
        let TopLevelDecl::Import(import) = declaration else { return true };
        !is_internal_module_path(&import.path, module_path)
    });
}

fn is_internal_module_path(import_path: &str, module_path: &str) -> bool {
    import_path == module_path
        || import_path.starts_with(&format!("{module_path}::"))
        || module_path.starts_with(&format!("{import_path}::"))
}

fn module_path_for_test(path: &Path, source_root: &Path, project_root: &Path) -> String {
    let source_path = if path.starts_with(source_root) {
        Some(path.to_path_buf())
    } else {
        let relative = path.strip_prefix(project_root.join("tests")).ok();
        relative.and_then(|relative| {
            if !is_nested_test_path(relative) {
                return None;
            }
            let candidate = source_root.join(relative);
            candidate.is_file().then_some(candidate.clone()).or_else(|| {
                let stem = candidate.file_stem()?.to_str()?;
                let facade = candidate.with_file_name(stem).join(format!("{stem}.act"));
                facade.is_file().then_some(facade)
            })
        })
    };
    let Some(source_path) = source_path else { return String::new() };
    module_source(&source_path, source_root).unwrap_or_default()
}

fn is_nested_test_path(path: &Path) -> bool {
    path.parent().is_some_and(|parent| !parent.as_os_str().is_empty())
}

fn module_source(path: &Path, source_root: &Path) -> Option<String> {
    let mut directory = path.parent()?.to_owned();
    let mut segments = Vec::new();
    while directory.starts_with(source_root) && directory != source_root {
        let module_name = directory.file_name()?.to_str()?.to_owned();
        if !directory.join(format!("{module_name}.act")).is_file() {
            break;
        }
        segments.push(module_name);
        directory = directory.parent()?.to_owned();
    }
    if segments.is_empty() {
        None
    } else {
        segments.reverse();
        Some(segments.join("::"))
    }
}

fn validate_semantics(
    path: &Path,
    source: &str,
    program: &Program,
    mode: ConformanceMode,
) -> Result<(), String> {
    if !mode.is_strict() {
        return Ok(());
    }
    crate::semantic::analyze(program).map(|_| ()).map_err(|error| {
        let diagnostic = semantic_diagnostic(&error).with_source_path(path.display().to_string());
        format!("{}: {}", path.display(), render_diagnostic(source, &diagnostic))
    })
}

fn discover_tests(
    path: &Path,
    module: &str,
    relative_path: &Path,
    program: Program,
    compilation_plan: Option<ModuleCompilationPlan>,
    names: &[String],
) -> Vec<DiscoveredTest> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb)
                if names.contains(&verb.name) && verb.metadata.contains(&MetaAttribute::Test) =>
            {
                Some(DiscoveredTest {
                    path: path.to_path_buf(),
                    module: if module.is_empty() { "<root>".to_owned() } else { module.to_owned() },
                    relative_path: relative_path.to_path_buf(),
                    name: verb.name.clone(),
                    program: program.clone(),
                    compilation_plan: compilation_plan.clone(),
                })
            }
            _ => None,
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

pub(super) fn collect_act_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !is_excluded_directory(&path) {
                collect_act_files(&path, paths);
            }
        } else if path.extension().is_some_and(|extension| extension == "act") {
            paths.push(path);
        }
    }
}

fn is_excluded_directory(path: &Path) -> bool {
    path.file_name().is_some_and(|name| matches!(name.to_str(), Some("fixtures" | "snapshots")))
}

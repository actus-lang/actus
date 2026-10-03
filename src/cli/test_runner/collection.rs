use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{MetaAttribute, Program, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{module_diagnostic, render_diagnostic, semantic_diagnostic};
use crate::lexer::{TokenKind, scan};
use crate::modules::{ModuleResolver, parse_module, resolve_imports};
use crate::parser::parse;
use crate::semantic::filter_program_for_target;

use super::super::conformance::{ConformanceMode, validate_source_limits};

pub(super) struct DiscoveredTest {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) program: Program,
    pub(super) source_program: Program,
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
    mode: ConformanceMode,
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
        let collected = collect_file_tests(&path, configuration, mode, &resolver)?;
        tests.extend(collected.tests);
        filtered += collected.filtered;
    }
    tests.sort_by(|left, right| left.path.cmp(&right.path).then(left.name.cmp(&right.name)));
    Ok(TestCollection { tests, filtered })
}

fn collect_file_tests(
    path: &Path,
    configuration: &CompilerConfiguration,
    mode: ConformanceMode,
    resolver: &ModuleResolver,
) -> Result<CollectedFile, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
    validate_source(path, &source, mode, configuration)?;
    let Some((source_program, program, test_names)) =
        parse_source(path, &source, resolver, configuration.source_root())?
    else {
        return Ok(CollectedFile { tests: Vec::new(), filtered: 0 });
    };
    let declared = test_count(&program);
    let program = filter_program_for_target(&program, configuration.target());
    let filtered = declared.saturating_sub(test_count(&program));
    validate_semantics(path, &source, &program, mode)?;
    Ok(CollectedFile {
        tests: discover_tests(path, source_program, program, &test_names),
        filtered,
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

fn parse_source(
    path: &Path,
    source: &str,
    resolver: &ModuleResolver,
    source_root: &Path,
) -> Result<Option<(Program, Program, Vec<String>)>, String> {
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
    let compilation_source = module_source(path, source_root)
        .map(|module_path| {
            parse_module(resolver, &module_path)
                .map_err(|error| format!("cannot load module `{module_path}`: {error}"))
        })
        .transpose()?
        .unwrap_or_else(|| source_program.clone());
    resolve_imports(&compilation_source, resolver)
        .map(|program| Some((compilation_source, program, test_names)))
        .map_err(|error| {
            let diagnostic = module_diagnostic(&error).with_source_path(path.display().to_string());
            format!("{}: {}", path.display(), render_diagnostic(source, &diagnostic))
        })
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
    segments.last().cloned()
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
    source_program: Program,
    program: Program,
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
                    name: verb.name.clone(),
                    program: program.clone(),
                    source_program: source_program.clone(),
                })
            }
            _ => None,
        })
        .collect()
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

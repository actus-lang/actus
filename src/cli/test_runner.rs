use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::ast::{Block, Expr, MetaAttribute, Program, Stmt, TopLevelDecl, TypeName, VerbDecl};
use crate::codegen::{emit_program_object_for_target, link_object};
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{render_diagnostic, semantic_diagnostic};
use crate::lexer::{SourceSpan, TokenKind, scan};
use crate::modules::{ModuleResolver, resolve_imports};
use crate::parser::parse;
use crate::semantic::filter_program_for_target;

use super::conformance::{ConformanceMode, parse_strict_option};

pub(super) fn test_command(arguments: impl Iterator<Item = String>) -> i32 {
    let mut mode = ConformanceMode::Standard;
    for argument in arguments {
        match parse_strict_option(&argument, &mut mode) {
            Ok(true) => continue,
            Ok(false) => {
                eprintln!("error: `actus test` does not accept argument `{argument}`");
                return 2;
            }
            Err(error) => {
                eprintln!("error: {error}");
                return 2;
            }
        }
    }
    let configuration = if mode.is_strict() {
        CompilerConfiguration::from_current_manifest_strict().map_err(|error| error.to_string())
    } else {
        CompilerConfiguration::from_current_manifest().map_err(|error| error.to_string())
    };
    let configuration = match configuration {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let mut files = Vec::new();
    collect_act_files(&configuration.project_root().join("tests"), &mut files);
    collect_act_files(configuration.source_root(), &mut files);
    files.sort();
    let tests = match collect_tests(files, &configuration, mode) {
        Ok(tests) => tests,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    run_tests(tests, &configuration, mode)
}

fn run_tests(
    tests: Vec<DiscoveredTest>,
    configuration: &CompilerConfiguration,
    mode: ConformanceMode,
) -> i32 {
    let output_style = TestOutputStyle::detect();
    if mode.is_strict() {
        println!("running {} tests (strict)", tests.len());
    } else {
        println!("running {} tests", tests.len());
    }
    let mut passed = 0;
    for (index, test) in tests.iter().enumerate() {
        let started = Instant::now();
        let result = run_test(test, configuration, index);
        let elapsed = started.elapsed().as_millis();
        match result {
            Ok(0) => {
                passed += 1;
                let status = output_style.success(&format!("ok ({} ms, exit code 0)", elapsed));
                println!("test {}::{} ... {}", test.path.display(), test.name, status);
            }
            Ok(code) => {
                let status =
                    output_style.failure(&format!("FAILED ({} ms, exit code {})", elapsed, code));
                println!("test {}::{} ... {}", test.path.display(), test.name, status);
            }
            Err(error) => {
                let status = output_style.failure(&format!("FAILED ({} ms, {})", elapsed, error));
                println!("test {}::{} ... {}", test.path.display(), test.name, status);
            }
        }
    }
    let failed = tests.len() - passed;
    let result =
        if failed == 0 { output_style.success("ok") } else { output_style.failure("FAILED") };
    println!("test result: {}. {} passed; {} failed", result, passed, failed);
    i32::from(failed != 0)
}

struct TestOutputStyle {
    enabled: bool,
}

impl TestOutputStyle {
    fn detect() -> Self {
        Self { enabled: io::stdout().is_terminal() }
    }

    fn success(&self, text: &str) -> String {
        self.paint("\x1b[32m", text)
    }

    fn failure(&self, text: &str) -> String {
        self.paint("\x1b[31m", text)
    }

    fn paint(&self, color: &str, text: &str) -> String {
        if self.enabled { format!("{color}{text}\x1b[0m") } else { text.to_owned() }
    }
}

struct DiscoveredTest {
    path: PathBuf,
    name: String,
    program: Program,
}

fn collect_tests(
    files: Vec<PathBuf>,
    configuration: &CompilerConfiguration,
    mode: ConformanceMode,
) -> Result<Vec<DiscoveredTest>, String> {
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    let mut tests = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
        let (tokens, errors) = scan(&source);
        if !tokens.iter().any(|token| matches!(token.kind, TokenKind::Meta)) {
            continue;
        }
        if !errors.is_empty() {
            return Err(format!("cannot lex `{}`", path.display()));
        }
        let program = parse(tokens)
            .map_err(|error| format!("cannot parse `{}`: {error:?}", path.display()))?;
        let program = resolve_imports(&program, &resolver)
            .map_err(|error| format!("cannot resolve `{}`: {error}", path.display()))?;
        let program = filter_program_for_target(&program, configuration.target());
        if mode.is_strict() {
            crate::semantic::analyze(&program).map_err(|error| {
                let diagnostic =
                    semantic_diagnostic(&error).with_source_path(path.display().to_string());
                format!("{}: {}", path.display(), render_diagnostic(&source, &diagnostic))
            })?;
        }
        for declaration in &program.declarations {
            if let TopLevelDecl::Verb(verb) = declaration
                && verb.metadata.contains(&MetaAttribute::Test)
            {
                tests.push(DiscoveredTest {
                    path: path.clone(),
                    name: verb.name.clone(),
                    program: program.clone(),
                });
            }
        }
    }
    tests.sort_by(|left, right| left.path.cmp(&right.path).then(left.name.cmp(&right.name)));
    Ok(tests)
}

fn run_test(
    test: &DiscoveredTest,
    configuration: &CompilerConfiguration,
    index: usize,
) -> Result<i32, String> {
    let Some(TopLevelDecl::Verb(verb)) = test.program.declarations.iter().find(|declaration| matches!(declaration, TopLevelDecl::Verb(candidate) if candidate.name == test.name)) else { return Err("test verb disappeared during collection".to_owned()); };
    if !verb.params.is_empty() {
        return Err("meta test verbs must not have parameters".to_owned());
    }
    let mut program = test.program.clone();
    if program
        .declarations
        .iter()
        .any(|declaration| matches!(declaration, TopLevelDecl::Verb(verb) if verb.name == "main"))
    {
        return Err("meta test program already defines `main`".to_owned());
    }
    program.declarations.push(test_main(&test.name));
    let object = emit_program_object_for_target(
        &program,
        "main",
        configuration.native_backend(),
        configuration.target(),
    )
    .map_err(|error| error.to_string())?;
    let root = std::env::temp_dir().join(format!("actus-test-{}-{index}", std::process::id()));
    let object_path = root.with_extension("o");
    let executable = root.with_extension("bin");
    fs::write(&object_path, object)
        .map_err(|error| format!("cannot write test object: {error}"))?;
    let result = link_object(&object_path, &executable, configuration)
        .map_err(|error| error.to_string())
        .and_then(|()| {
            Command::new(&executable)
                .stdin(Stdio::null())
                .status()
                .map_err(|error| error.to_string())
        });
    let _ = fs::remove_file(&object_path);
    let _ = fs::remove_file(&executable);
    result.map(|status| status.code().unwrap_or(1))
}

fn test_main(name: &str) -> TopLevelDecl {
    let span = SourceSpan::new(0, 0);
    TopLevelDecl::Verb(VerbDecl {
        is_open: false,
        doc: None,
        metadata: Vec::new(),
        name: "main".to_owned(),
        generic_parameters: Vec::new(),
        params: Vec::new(),
        return_type: Some(crate::ast::ReturnType {
            access: crate::ast::ReturnAccess::Owned,
            ty: TypeName {
                name: "Int".to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span,
            },
            span,
        }),
        body: Block {
            statements: vec![Stmt::Return {
                value: Some(Expr::Call { callee: name.to_owned(), arguments: Vec::new(), span }),
                span,
            }],
            span,
        },
        span,
    })
}

fn collect_act_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let excluded = path
                .file_name()
                .is_some_and(|name| matches!(name.to_str(), Some("fixtures" | "snapshots")));
            if !excluded {
                collect_act_files(&path, paths);
            }
        } else if path.extension().is_some_and(|extension| extension == "act") {
            paths.push(path);
        }
    }
}

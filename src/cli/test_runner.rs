use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::ast::{Block, Expr, MetaAttribute, Program, Stmt, TopLevelDecl, TypeName, VerbDecl};
use crate::codegen::{emit_program_object_for_target, link_object};
use crate::configuration::CompilerConfiguration;
use crate::lexer::{SourceSpan, TokenKind, scan};
use crate::modules::{ModuleResolver, resolve_imports};
use crate::parser::parse;

pub(super) fn test_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    if arguments.next().is_some() {
        eprintln!("error: `actus test` does not accept positional arguments yet");
        return 2;
    }
    let configuration = match CompilerConfiguration::from_current_manifest() {
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
    let tests = match collect_tests(files, &configuration) {
        Ok(tests) => tests,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    run_tests(tests, &configuration)
}

fn run_tests(tests: Vec<DiscoveredTest>, configuration: &CompilerConfiguration) -> i32 {
    println!("running {} tests", tests.len());
    let mut passed = 0;
    for (index, test) in tests.iter().enumerate() {
        let started = Instant::now();
        let result = run_test(test, configuration, index);
        let elapsed = started.elapsed().as_millis();
        match result {
            Ok(0) => {
                passed += 1;
                println!(
                    "test {}::{} ... ok ({} ms, exit code 0)",
                    test.path.display(),
                    test.name,
                    elapsed
                );
            }
            Ok(code) => println!(
                "test {}::{} ... FAILED ({} ms, exit code {})",
                test.path.display(),
                test.name,
                elapsed,
                code
            ),
            Err(error) => println!(
                "test {}::{} ... FAILED ({} ms, {})",
                test.path.display(),
                test.name,
                elapsed,
                error
            ),
        }
    }
    let failed = tests.len() - passed;
    println!(
        "test result: {}. {} passed; {} failed",
        if failed == 0 { "ok" } else { "FAILED" },
        passed,
        failed
    );
    i32::from(failed != 0)
}

struct DiscoveredTest {
    path: PathBuf,
    name: String,
    program: Program,
}

fn collect_tests(
    files: Vec<PathBuf>,
    configuration: &CompilerConfiguration,
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
        .and_then(|()| Command::new(&executable).status().map_err(|error| error.to_string()));
    let _ = fs::remove_file(&object_path);
    let _ = fs::remove_file(&executable);
    result.map(|status| status.code().unwrap_or(1))
}

fn test_main(name: &str) -> TopLevelDecl {
    let span = SourceSpan::new(0, 0);
    TopLevelDecl::Verb(VerbDecl {
        is_open: false,
        metadata: Vec::new(),
        name: "main".to_owned(),
        generic_parameters: Vec::new(),
        params: Vec::new(),
        return_type: Some(crate::ast::ReturnType {
            access: crate::ast::ReturnAccess::Owned,
            ty: TypeName { name: "Int".to_owned(), arguments: Vec::new(), span },
            span,
        }),
        body: Block {
            statements: vec![
                Stmt::Expression {
                    expression: Expr::Call { callee: name.to_owned(), arguments: Vec::new(), span },
                    span,
                },
                Stmt::Return { value: Some(Expr::Integer { value: "0".to_owned(), span }), span },
            ],
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

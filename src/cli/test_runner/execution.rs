use std::fs;
use std::time::Instant;

use crate::ast::{Block, Expr, Program, Stmt, TopLevelDecl, TypeName, VerbDecl};
use crate::codegen::{emit_program_object_for_target, link_object};
use crate::configuration::CompilerConfiguration;
use crate::lexer::SourceSpan;

use super::super::conformance::ConformanceMode;
use super::collection::{DiscoveredTest, TestCollection};
use super::output::TestOutputStyle;
use super::process::{TestProcessResult, run_test_process};

pub(super) fn run_tests(
    collection: TestCollection,
    configuration: &CompilerConfiguration,
    mode: ConformanceMode,
) -> i32 {
    let output_style = TestOutputStyle::detect();
    print_test_header(collection.tests.len(), collection.filtered, mode);
    let mut passed = 0;
    for (index, test) in collection.tests.iter().enumerate() {
        let started = Instant::now();
        let result = run_test(test, configuration, index);
        if report_test_result(test, result, started, &output_style) {
            passed += 1;
        }
    }
    print_test_summary(collection.tests.len(), passed, &output_style)
}

fn print_test_header(total: usize, filtered: usize, mode: ConformanceMode) {
    if mode.is_strict() {
        println!("running {total} tests (strict) ({filtered} filtered)");
    } else {
        println!("running {total} tests ({filtered} filtered)");
    }
}

fn report_test_result(
    test: &DiscoveredTest,
    result: Result<TestProcessResult, String>,
    started: Instant,
    output_style: &TestOutputStyle,
) -> bool {
    let elapsed = started.elapsed().as_millis();
    match result {
        Ok(TestProcessResult::Exited(0)) => {
            let status = output_style.success(&format!("ok ({elapsed} ms, exit code 0)"));
            println!("test {}::{} ... {status}", test.path.display(), test.name);
            true
        }
        Ok(TestProcessResult::Exited(code)) => {
            let status = output_style.failure(&format!("FAILED ({elapsed} ms, exit code {code})"));
            println!("test {}::{} ... {status}", test.path.display(), test.name);
            false
        }
        Ok(TestProcessResult::Signaled) => {
            let status =
                output_style.failure(&format!("FAILED ({elapsed} ms, terminated by signal)"));
            println!("test {}::{} ... {status}", test.path.display(), test.name);
            false
        }
        Err(error) => {
            let status = output_style.failure(&format!("FAILED ({elapsed} ms, {error})"));
            println!("test {}::{} ... {status}", test.path.display(), test.name);
            false
        }
    }
}

fn print_test_summary(total: usize, passed: usize, output_style: &TestOutputStyle) -> i32 {
    let failed = total - passed;
    let result =
        if failed == 0 { output_style.success("ok") } else { output_style.failure("FAILED") };
    println!("test result: {result}. {passed} passed; {failed} failed");
    i32::from(failed != 0)
}

fn run_test(
    test: &DiscoveredTest,
    configuration: &CompilerConfiguration,
    index: usize,
) -> Result<TestProcessResult, String> {
    let verb = test_verb(test)?;
    validate_test_verb(verb)?;
    let program = test_program(test)?;
    let object = emit_program_object_for_target(
        &program,
        "main",
        configuration.native_backend(),
        configuration.target(),
    )
    .map_err(|error| error.to_string())?;
    let paths = TestArtifactPaths::new(index);
    let result = fs::write(&paths.object, object)
        .map_err(|error| format!("cannot write test object: {error}"))
        .and_then(|()| {
            link_object(&paths.object, &paths.executable, configuration)
                .map_err(|error| error.to_string())
        })
        .and_then(|()| run_test_process(&paths.executable));
    match (result, paths.cleanup()) {
        (Ok(process), Ok(())) => Ok(process),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(run_error), Err(cleanup_error)) => Err(format!("{run_error}; {cleanup_error}")),
    }
}

fn test_verb(test: &DiscoveredTest) -> Result<&crate::ast::VerbDecl, String> {
    test.program
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) if verb.name == test.name => Some(verb),
            _ => None,
        })
        .ok_or_else(|| "test verb disappeared during collection".to_owned())
}

fn validate_test_verb(verb: &crate::ast::VerbDecl) -> Result<(), String> {
    if !verb.params.is_empty() {
        return Err("meta test verbs must not have parameters".to_owned());
    }
    Ok(())
}

fn test_program(test: &DiscoveredTest) -> Result<Program, String> {
    let mut program = test.program.clone();
    if program
        .declarations
        .iter()
        .any(|declaration| matches!(declaration, TopLevelDecl::Verb(candidate) if candidate.name == "main"))
    {
        return Err("meta test program already defines `main`".to_owned());
    }
    program.declarations.push(test_main(&test.name));
    Ok(program)
}

struct TestArtifactPaths {
    object: std::path::PathBuf,
    executable: std::path::PathBuf,
}

impl TestArtifactPaths {
    fn new(index: usize) -> Self {
        let root = std::env::temp_dir().join(format!("actus-test-{}-{index}", std::process::id()));
        Self { object: root.with_extension("o"), executable: root.with_extension("bin") }
    }

    fn cleanup(&self) -> Result<(), String> {
        remove_artifact(&self.object)?;
        remove_artifact(&self.executable)
    }
}

fn remove_artifact(path: &std::path::Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot clean up `{}`: {error}", path.display())),
    }
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

use std::fs;
use std::time::Instant;

use crate::ast::{Block, Expr, Stmt, TopLevelDecl, TypeName, VerbDecl};
use crate::codegen::link_objects;
use crate::configuration::CompilerConfiguration;
use crate::lexer::SourceSpan;
use crate::modules::{ModuleResolver, build_compilation_plan};

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
    let plan = test_compilation_plan(test, configuration)?;
    let objects = crate::cli::build::emit_objects(&plan, "main", configuration)
        .map_err(|error| error.to_string())?;
    let mut paths = TestArtifactPaths::new(index);
    let result = paths
        .write_objects(&objects)
        .and_then(|()| {
            let references =
                paths.objects.iter().map(std::path::PathBuf::as_path).collect::<Vec<_>>();
            link_objects(&references, &paths.executable, configuration)
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

fn test_compilation_plan(
    test: &DiscoveredTest,
    configuration: &CompilerConfiguration,
) -> Result<crate::modules::ModuleCompilationPlan, String> {
    let mut program = test_program(&test.source_program, &test.name);
    if program
        .declarations
        .iter()
        .any(|declaration| matches!(declaration, TopLevelDecl::Verb(candidate) if candidate.name == "main"))
    {
        return Err("meta test program already defines `main`".to_owned());
    }
    program.declarations.push(test_main(&test.name));
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    );
    build_compilation_plan(&program, &resolver).map_err(|error| error.to_string())
}

fn test_program(source_program: &crate::ast::Program, test_name: &str) -> crate::ast::Program {
    let mut program = source_program.clone();
    program.declarations.retain(|declaration| {
        !matches!(
            declaration,
            TopLevelDecl::Verb(verb)
                if verb.metadata.contains(&crate::ast::MetaAttribute::Test)
                    && verb.name != test_name
        )
    });
    program
}

struct TestArtifactPaths {
    objects: Vec<std::path::PathBuf>,
    executable: std::path::PathBuf,
}

impl TestArtifactPaths {
    fn new(index: usize) -> Self {
        let root = std::env::temp_dir().join(format!("actus-test-{}-{index}", std::process::id()));
        Self { objects: Vec::new(), executable: root.with_extension("bin") }
    }

    fn write_objects(
        &mut self,
        objects: &[crate::cli::build::EmittedObject],
    ) -> Result<(), String> {
        let root = self.executable.with_extension("root.o");
        for (index, object) in objects.iter().enumerate() {
            let path = if index == 0 {
                root.clone()
            } else {
                self.executable.with_file_name(format!(
                    "{}.module.{}.o",
                    self.executable
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .unwrap_or("actus-test"),
                    object.name.replace([':', '/', '\\'], "_")
                ))
            };
            fs::write(&path, &object.bytes)
                .map_err(|error| format!("cannot write test object: {error}"))?;
            self.objects.push(path);
        }
        Ok(())
    }

    fn cleanup(&self) -> Result<(), String> {
        for object in &self.objects {
            remove_artifact(object)?;
        }
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

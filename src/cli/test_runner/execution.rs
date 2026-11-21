use std::fs;
use std::sync::mpsc;
use std::time::Instant;

use crate::ast::{
    Argument, ArgumentRoleResolution, Block, Expr, Role, Stmt, TestFault, TopLevelDecl, TypeName,
    VerbDecl,
};
#[cfg(feature = "test-fault-injection")]
use crate::ast::{DispatchMode, ExternalVerbDecl, ForeignAbi, Param};
use crate::codegen::link_objects;
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{render_diagnostic, semantic_diagnostic};
use crate::lexer::SourceSpan;
use crate::modules::{ModuleResolver, build_test_compilation_plan};

use super::super::conformance::ConformanceMode;
use super::TestOptions;
use super::collection::{DiscoveredTest, TestCollection};
use super::output::TestOutputStyle;
use super::process::{TestProcessResult, run_test_process};

pub(super) fn run_tests(
    collection: TestCollection,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
) -> i32 {
    let output_style = TestOutputStyle::detect();
    print_test_header(collection.tests.len(), collection.filtered, options.mode);
    if options.list {
        for test in &collection.tests {
            println!("{}::{}", test.display_module, test.name);
        }
        return 0;
    }
    let run_started = Instant::now();
    if collection.tests.is_empty() {
        return print_test_summary(0, 0, run_started.elapsed(), &output_style);
    }
    let worker_count = options.test_threads.min(collection.tests.len());
    let chunk_size = collection.tests.len().div_ceil(worker_count);
    let tests = &collection.tests;
    let (sender, receiver) = mpsc::channel();
    let passed = std::thread::scope(|scope| {
        let handles = (0..collection.tests.len())
            .step_by(chunk_size)
            .map(|start| {
                let end = (start + chunk_size).min(collection.tests.len());
                let sender = sender.clone();
                scope.spawn(move || {
                    for (offset, test) in tests[start..end].iter().enumerate() {
                        let index = start + offset;
                        let started = Instant::now();
                        let result = run_test(test, configuration, index);
                        sender
                            .send((index, result, started.elapsed()))
                            .expect("test result receiver dropped");
                    }
                })
            })
            .collect::<Vec<_>>();
        drop(sender);
        let mut passed = 0;
        for (index, result, elapsed) in receiver {
            if report_test_result(&collection.tests[index], result, elapsed, &output_style) {
                passed += 1;
            }
        }
        for handle in handles {
            handle.join().expect("test execution worker panicked");
        }
        passed
    });
    print_test_summary(collection.tests.len(), passed, run_started.elapsed(), &output_style)
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
    elapsed: std::time::Duration,
    output_style: &TestOutputStyle,
) -> bool {
    let elapsed = elapsed.as_millis();
    let label = format!("test {}::{}", test.display_module, test.name);
    match result {
        Ok(TestProcessResult::Exited { code: 0, .. }) => {
            let status = output_style.success(&format!("ok ({elapsed} ms, exit code 0)"));
            println!("{label} ... {status}");
            true
        }
        Ok(TestProcessResult::Exited { code, stderr }) => {
            let mut detail = format!("FAILED ({elapsed} ms, exit code {code})");
            super::process::append_stderr_for_report(&mut detail, &stderr);
            let status = output_style.failure(&detail);
            println!("{label} ... {status}");
            false
        }
        Ok(TestProcessResult::Signaled { signal, stderr }) => {
            if matches!(
                test.fault,
                Some(TestFault::InterruptedBeforeRename | TestFault::InterruptedPublication)
            ) {
                let status =
                    output_style.success(&format!("ok ({elapsed} ms, expected interruption)"));
                println!("{label} ... {status}");
                return true;
            }
            let signal = signal.map_or_else(|| "unknown".to_owned(), |value| value.to_string());
            let mut detail = format!("FAILED ({elapsed} ms, terminated by signal {signal})");
            super::process::append_stderr_for_report(&mut detail, &stderr);
            let status = output_style.failure(&detail);
            println!("{label} ... {status}");
            false
        }
        Err(error) => {
            let status = output_style.failure(&format!("FAILED ({elapsed} ms, {error})"));
            println!("{label} ... {status}");
            false
        }
    }
}

fn print_test_summary(
    total: usize,
    passed: usize,
    elapsed: std::time::Duration,
    output_style: &TestOutputStyle,
) -> i32 {
    let failed = total - passed;
    let result =
        if failed == 0 { output_style.success("ok") } else { output_style.failure("FAILED") };
    println!(
        "test result: {result}. {passed} passed; {failed} failed; finished in {}",
        format_duration(elapsed)
    );
    i32::from(failed != 0)
}

fn format_duration(duration: std::time::Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1_000 {
        return format!("{millis} ms");
    }
    let seconds = millis / 1_000;
    if seconds < 60 {
        return format!("{}.{:01} s", seconds, (millis % 1_000) / 100);
    }
    format!("{}m {:02}.{:01}s", seconds / 60, seconds % 60, (millis % 1_000) / 100)
}

fn run_test(
    test: &DiscoveredTest,
    configuration: &CompilerConfiguration,
    index: usize,
) -> Result<TestProcessResult, String> {
    let verb = test_verb(test)?;
    validate_test_verb(verb)?;
    let plan = test_compilation_plan(test, configuration)?;
    let caller =
        crate::semantic::filter_program_for_target_checked(plan.caller(), configuration.target())
            .map_err(|error| test_diagnostic(test, semantic_diagnostic(&error)))?;
    crate::semantic::analyze_resolved(&caller)
        .map_err(|error| test_diagnostic(test, semantic_diagnostic(&error)))?;
    crate::modules::validate_compilation_plan(&plan, configuration.target())
        .map_err(|error| test_diagnostic(test, crate::diagnostics::module_diagnostic(&error)))?;
    let object = crate::cli::build::emit_object(&plan, "main", configuration, None)
        .map_err(|error| error.to_string())?;
    let paths = TestArtifactPaths::new(index)?;
    let result = fs::write(&paths.object, &object.bytes)
        .map_err(|error| format!("cannot write test object: {error}"))
        .and_then(|()| {
            link_objects(&[paths.object.as_path()], &paths.executable, configuration)
                .map_err(|error| error.to_string())
        })
        .and_then(|()| run_test_process(&paths.executable, &paths.working_directory));
    match (result, paths.cleanup()) {
        (Ok(process), Ok(())) => Ok(process),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(run_error), Err(cleanup_error)) => Err(format!("{run_error}; {cleanup_error}")),
    }
}

fn test_diagnostic(test: &DiscoveredTest, diagnostic: crate::diagnostics::Diagnostic) -> String {
    test.source_map
        .lock()
        .localize(&diagnostic)
        .map(|(path, source, diagnostic)| {
            format!("{}: {}", path.display(), render_diagnostic(&source, &diagnostic))
        })
        .unwrap_or_else(|| {
            format!("{}: {}", test.path.display(), render_diagnostic("", &diagnostic))
        })
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
    let mut program = test_program(&test.program, &test.name);
    let test_contract = test_verb(test)?
        .metadata
        .iter()
        .filter_map(|attribute| match attribute {
            crate::ast::MetaAttribute::Effect(_) | crate::ast::MetaAttribute::Capability(_) => {
                Some(attribute.clone())
            }
            _ => None,
        })
        .collect();
    if program
        .declarations
        .iter()
        .any(|declaration| matches!(declaration, TopLevelDecl::Verb(candidate) if candidate.name == "main"))
    {
        return Err("meta test program already defines `main`".to_owned());
    }
    if let Some(fault) = test.fault {
        #[cfg(not(feature = "test-fault-injection"))]
        return Err(format!(
            "test `{}` declares fault `{}`; rebuild Actus with `--features test-fault-injection`",
            test.name,
            fault.name()
        ));
        #[cfg(feature = "test-fault-injection")]
        {
            let _ = fault;
            program.declarations.push(test_fault_declaration());
        }
    }
    program.declarations.push(test_main(&test.name, test.fault, test_contract));
    if let Some(plan) = &test.compilation_plan {
        return Ok(plan.with_caller_program(program));
    }
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_package_identity(configuration.package_identity(), configuration.source_root());
    build_test_compilation_plan(&program, &resolver).map_err(|error| error.to_string())
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
    object: std::path::PathBuf,
    executable: std::path::PathBuf,
    working_directory: std::path::PathBuf,
}

impl TestArtifactPaths {
    fn new(index: usize) -> Result<Self, String> {
        let root = std::env::temp_dir().join(format!("actus-test-{}-{index}", std::process::id()));
        fs::create_dir_all(&root)
            .map_err(|error| format!("cannot create test working directory: {error}"))?;
        let executable = root.join("test.bin");
        Ok(Self { object: executable.with_extension("o"), executable, working_directory: root })
    }

    fn cleanup(&self) -> Result<(), String> {
        if std::env::var("ACTUS_TEST_KEEP_ARTIFACTS").as_deref() == Ok("1") {
            return Ok(());
        }
        remove_artifact(&self.object)?;
        remove_artifact(&self.executable)?;
        fs::remove_dir_all(&self.working_directory).map_err(|error| {
            format!(
                "cannot clean up test working directory `{}`: {error}",
                self.working_directory.display()
            )
        })
    }
}

fn remove_artifact(path: &std::path::Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot clean up `{}`: {error}", path.display())),
    }
}

#[cfg(feature = "test-fault-injection")]
fn test_fault_declaration() -> TopLevelDecl {
    let span = SourceSpan::new(0, 0);
    TopLevelDecl::ExternalVerb(ExternalVerbDecl {
        is_open: false,
        doc: None,
        contract: None,
        unsafe_boundary: true,
        module_import: false,
        abi: ForeignAbi::C,
        metadata: Vec::new(),
        name: "actus_test_publication_fault".to_owned(),
        native_symbol: None,
        generic_parameters: Vec::new(),
        params: vec![Param {
            role: Role::Abs,
            name: "code".to_owned(),
            dispatch: DispatchMode::Static,
            ty: TypeName::builtin("Int", Vec::new(), None, span),
            span,
        }],
        return_type: Some(crate::ast::ReturnType {
            access: crate::ast::ReturnAccess::Owned,
            ty: TypeName::builtin("Int", Vec::new(), None, span),
            span,
        }),
        span,
        identity: None,
    })
}

fn test_main(
    name: &str,
    fault: Option<TestFault>,
    metadata: Vec<crate::ast::MetaAttribute>,
) -> TopLevelDecl {
    let span = SourceSpan::new(0, 0);
    let mut statements = Vec::new();
    if let Some(fault) = fault {
        statements.push(Stmt::Expression {
            expression: Expr::Call {
                callee: "actus_test_publication_fault".to_owned(),
                resolved_type: None,
                generic_arguments: Vec::new(),
                arguments: vec![Argument {
                    name: None,
                    role: Some(Role::Abs),
                    role_span: None,
                    role_resolution: ArgumentRoleResolution::Explicit,
                    expression: Expr::Integer {
                        value: fault.code().to_string(),
                        suffix: None,
                        span,
                    },
                }],
                span,
            },
            span,
        });
    }
    statements.push(Stmt::Return {
        value: Some(Expr::Call {
            callee: name.to_owned(),
            resolved_type: None,
            generic_arguments: Vec::new(),
            arguments: Vec::new(),
            span,
        }),
        span,
    });
    TopLevelDecl::Verb(VerbDecl {
        is_open: false,
        doc: None,
        contract: None,
        metadata,
        name: "main".to_owned(),
        generic_parameters: Vec::new(),
        params: Vec::new(),
        return_type: Some(crate::ast::ReturnType {
            access: crate::ast::ReturnAccess::Owned,
            ty: TypeName::builtin("Int", Vec::new(), None, span),
            span,
        }),
        body: Block { statements, span },
        span,
        identity: None,
    })
}

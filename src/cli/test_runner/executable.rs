//! Building the executable of one build unit: the frontend, code generation,
//! object emission and linking of a test directory's program with its
//! generated entry (ADR-0121).

use std::fs;
use std::path::PathBuf;

use crate::codegen::link_objects;
use crate::configuration::{CompilerConfiguration, EntryContract};
use crate::diagnostics::{render_diagnostic, semantic_diagnostic};

use super::collection::{DiscoveredTest, TestGroup};
use super::dispatcher::append_entry;

const KEEP_ARTIFACTS_ENV: &str = "ACTUS_TEST_KEEP_ARTIFACTS";

/// A linked test executable. `directory` holds it and the working directories
/// of the tests that run it.
pub(super) struct TestExecutable {
    pub(super) directory: PathBuf,
    pub(super) executable: PathBuf,
}

pub(super) fn keep_artifacts() -> bool {
    std::env::var(KEEP_ARTIFACTS_ENV).as_deref() == Ok("1")
}

/// Compiles the program of `group` with the entry of `tests` to one object:
/// the frontend checks, code generation and object emission. The entry is
/// added to a copy of the group's program, so the group stays reusable.
fn emit_unit_object(
    group: &TestGroup,
    tests: &[(usize, &DiscoveredTest)],
    dispatch: bool,
    configuration: &CompilerConfiguration,
) -> Result<Vec<u8>, String> {
    let diagnostic = |diagnostic: crate::diagnostics::Diagnostic| {
        group
            .source_map
            .lock()
            .localize(&diagnostic)
            .map(|(path, source, diagnostic)| {
                format!("{}: {}", path.display(), render_diagnostic(&source, &diagnostic))
            })
            .unwrap_or_else(|| {
                format!("{}: {}", tests[0].1.path.display(), render_diagnostic("", &diagnostic))
            })
    };
    let mut program = group.program.clone();
    append_entry(&mut program, tests, dispatch)?;
    let plan = group.compilation_plan.with_caller_program(program);
    let caller =
        crate::semantic::filter_program_for_target_checked(plan.caller(), configuration.target())
            .map_err(|error| diagnostic(semantic_diagnostic(&error)))?;
    crate::semantic::analyze_resolved(&caller)
        .map_err(|error| diagnostic(semantic_diagnostic(&error)))?;
    crate::modules::validate_compilation_plan(&plan, configuration.target())
        .map_err(|error| diagnostic(crate::diagnostics::module_diagnostic(&error)))?;
    // A hosted test executable reports a failed runtime check with its source
    // location, as `actus build` does (ADR-0117); freestanding tests trap.
    let check_locations = matches!(configuration.entry_contract(), EntryContract::Hosted)
        .then_some(&group.source_map);
    crate::cli::build::emit_object(&plan, "main", configuration, check_locations)
        .map(|object| object.bytes)
        .map_err(|error| error.to_string())
}

/// Builds the executable that runs `tests` of `group`; each test is paired with
/// the index that selects it. `dispatch` is false for a single test that needs
/// no selection (freestanding targets). `unit` numbers the executable of this
/// run. A failure carries the rendered diagnostic.
pub(super) fn build_executable(
    group: &TestGroup,
    tests: &[(usize, &DiscoveredTest)],
    dispatch: bool,
    unit: usize,
    configuration: &CompilerConfiguration,
) -> Result<TestExecutable, String> {
    let object = emit_unit_object(group, tests, dispatch, configuration)?;
    let directory = std::env::temp_dir().join(format!("actus-test-{}-{unit}", std::process::id()));
    fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create test working directory: {error}"))?;
    let built = TestExecutable { executable: directory.join("test.bin"), directory };
    let object_path = built.executable.with_extension("o");
    let linked = fs::write(&object_path, &object)
        .map_err(|error| format!("cannot write test object: {error}"))
        .and_then(|()| {
            link_objects(&[object_path.as_path()], &built.executable, configuration)
                .map_err(|error| error.to_string())
        });
    if keep_artifacts() {
        return linked.map(|()| built);
    }
    let _ = fs::remove_file(&object_path);
    match linked {
        Ok(()) => Ok(built),
        Err(error) => {
            let _ = fs::remove_dir_all(&built.directory);
            Err(error)
        }
    }
}

impl TestExecutable {
    /// A fresh, empty working directory for the test at `position`.
    pub(super) fn working_directory(&self, position: usize) -> Result<PathBuf, String> {
        let directory = self.directory.join(format!("run-{position}"));
        fs::create_dir_all(&directory)
            .map_err(|error| format!("cannot create test working directory: {error}"))?;
        Ok(directory)
    }
}

/// Removes `path` and everything under it unless artifacts are kept.
pub(super) fn remove_artifacts(path: &std::path::Path) -> Result<(), String> {
    if keep_artifacts() {
        return Ok(());
    }
    fs::remove_dir_all(path)
        .map_err(|error| format!("cannot clean up test directory `{}`: {error}", path.display()))
}

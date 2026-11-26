//! Scheduling of `actus test` (ADR-0121): one executable per build unit, one
//! process per test.
//!
//! A build unit is the group of a test directory on hosted targets and a
//! single test on freestanding targets, whose executables cannot select a test
//! at start. `--test-threads` workers share one queue of jobs: building a
//! unit's executable and running one of its tests. A finished build queues the
//! unit's tests, and running is preferred to building so executables are
//! removed early. Results are reported in canonical test order.

use crate::cli::terminal_output::{OutputStopped, try_outln};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::ast::{MetaAttribute, TopLevelDecl};
use crate::configuration::{CompilerConfiguration, EntryContract};

use super::TestOptions;
use super::collection::{DiscoveredTest, TestCollection};
use super::executable::{TestExecutable, build_executable, remove_artifacts};
use super::output::TestOutputStyle;
use super::process::{TestProcessResult, run_test_process};
use super::report::{print_test_header, print_test_summary, report_test_result};

type TestResult = Result<TestProcessResult, String>;

pub(super) fn run_tests(
    collection: TestCollection,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
) -> i32 {
    match report_tests(collection, configuration, options) {
        Ok(status) => status,
        // The reader of the report went away: the run has been stopped and its
        // artifacts removed, so all that is left is the status.
        Err(OutputStopped(status)) => status,
    }
}

fn report_tests(
    collection: TestCollection,
    configuration: &CompilerConfiguration,
    options: &TestOptions,
) -> Result<i32, OutputStopped> {
    let output_style = TestOutputStyle::detect();
    let total = collection.order.len();
    print_test_header(total, collection.filtered, options.mode, configuration.profile())?;
    if options.list {
        for position in &collection.order {
            let test = collection.test(*position);
            try_outln!("{}::{}", test.display_module, test.name)?;
        }
        return Ok(0);
    }
    let run_started = Instant::now();
    if total == 0 {
        return print_test_summary(0, 0, run_started.elapsed(), &output_style);
    }
    let passed = execute_tests(&collection, configuration, options.test_threads, &output_style)?;
    print_test_summary(total, passed, run_started.elapsed(), &output_style)
}

/// Builds and runs every selected test and reports the results in canonical
/// order. Returns the number of tests that passed. When the report cannot be
/// written any more, the workers stop taking jobs, the jobs in progress finish
/// and every executable that was built is removed before the error returns.
fn execute_tests(
    collection: &TestCollection,
    configuration: &CompilerConfiguration,
    test_threads: usize,
    output_style: &TestOutputStyle,
) -> Result<usize, OutputStopped> {
    let total = collection.order.len();
    let schedule = Schedule::new(collection, configuration);
    let (sender, receiver) = mpsc::channel();
    for (flat, defect) in &schedule.defects {
        let _ = sender.send((*flat, Err(defect.clone()), Duration::ZERO));
    }
    let context = Context::new(collection, configuration, schedule);
    let reported = std::thread::scope(|scope| {
        let handles = (0..test_threads.min(total))
            .map(|_| {
                let sender = sender.clone();
                let context = &context;
                scope.spawn(move || context.work(&sender))
            })
            .collect::<Vec<_>>();
        drop(sender);
        let reported = report_in_order(&receiver, collection, output_style);
        if reported.is_err() {
            context.cancel();
        }
        for handle in handles {
            handle.join().expect("test execution worker panicked");
        }
        reported
    });
    if reported.is_err() {
        // Tests that never ran leave their executables behind otherwise.
        for executable in context.executables.iter().filter_map(OnceLock::get) {
            let _ = remove_artifacts(&executable.directory);
        }
    }
    reported
}

/// Reports results as they arrive, in canonical test order: a result waits
/// until every test before it has been reported. Returns the passed count, or
/// stops at the first result that cannot be written.
fn report_in_order(
    receiver: &Receiver<(usize, TestResult, Duration)>,
    collection: &TestCollection,
    output_style: &TestOutputStyle,
) -> Result<usize, OutputStopped> {
    let total = collection.order.len();
    let mut pending: Vec<Option<(TestResult, Duration)>> = (0..total).map(|_| None).collect();
    let (mut next, mut passed) = (0, 0);
    for (flat, result, elapsed) in receiver {
        pending[flat] = Some((result, elapsed));
        while let Some(Some((result, elapsed))) = pending.get_mut(next).map(Option::take) {
            let test = collection.test(collection.order[next]);
            passed += usize::from(report_test_result(test, result, elapsed, output_style)?);
            next += 1;
        }
    }
    Ok(passed)
}

/// One executable to build and the tests it runs.
struct BuildUnit {
    group: usize,
    /// Positions in the group of the tests the executable holds, which are
    /// also the indices that select them.
    positions: Vec<usize>,
    dispatch: bool,
}

struct Schedule {
    units: Vec<BuildUnit>,
    /// Canonical index of every test, by group and position.
    flat: Vec<Vec<usize>>,
    /// Tests that cannot run at all, by canonical index, with the reason.
    defects: Vec<(usize, String)>,
}

impl Schedule {
    fn new(collection: &TestCollection, configuration: &CompilerConfiguration) -> Self {
        let hosted = matches!(configuration.entry_contract(), EntryContract::Hosted);
        let mut flat: Vec<Vec<usize>> =
            collection.groups.iter().map(|group| vec![0; group.tests.len()]).collect();
        for (index, (group, position)) in collection.order.iter().enumerate() {
            flat[*group][*position] = index;
        }
        let mut group_unit: Vec<Option<usize>> = vec![None; collection.groups.len()];
        let mut units: Vec<BuildUnit> = Vec::new();
        let mut defects = Vec::new();
        // Units follow the canonical order so early results arrive early.
        for (index, (group, position)) in collection.order.iter().enumerate() {
            let test = collection.test((*group, *position));
            if let Some(defect) = test_defect(&collection.groups[*group].program, test) {
                defects.push((index, defect));
            } else if let (true, Some(unit)) = (hosted, group_unit[*group]) {
                units[unit].positions.push(*position);
            } else {
                group_unit[*group] = Some(units.len());
                units.push(BuildUnit {
                    group: *group,
                    positions: vec![*position],
                    dispatch: hosted,
                });
            }
        }
        for unit in &mut units {
            unit.positions.sort_unstable();
        }
        Self { units, flat, defects }
    }
}

/// Why a test cannot be run, if it cannot. Such a test is left out of its
/// executable and fails alone.
fn test_defect(program: &crate::ast::Program, test: &DiscoveredTest) -> Option<String> {
    let verb = program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Verb(verb)
            if verb.name == test.name && verb.metadata.contains(&MetaAttribute::Test) =>
        {
            Some(verb)
        }
        _ => None,
    });
    let Some(verb) = verb else {
        return Some("test verb disappeared during collection".to_owned());
    };
    if !verb.params.is_empty() {
        return Some("meta test verbs must not have parameters".to_owned());
    }
    #[cfg(not(feature = "test-fault-injection"))]
    if let Some(fault) = test.fault {
        return Some(format!(
            "test `{}` declares fault `{}`; rebuild Actus with `--features test-fault-injection`",
            test.name,
            fault.name()
        ));
    }
    None
}

enum Job {
    Build(usize),
    Run { unit: usize, position: usize },
}

struct Queue {
    builds: VecDeque<usize>,
    runs: VecDeque<(usize, usize)>,
    building: usize,
}

/// What the workers share.
struct Context<'a> {
    collection: &'a TestCollection,
    configuration: &'a CompilerConfiguration,
    schedule: Schedule,
    queue: Mutex<Queue>,
    changed: Condvar,
    /// Set when the report cannot be written; workers take no further job.
    cancelled: AtomicBool,
    executables: Vec<OnceLock<TestExecutable>>,
    /// Tests of each unit that have not finished yet.
    remaining: Vec<AtomicUsize>,
}

impl<'a> Context<'a> {
    fn new(
        collection: &'a TestCollection,
        configuration: &'a CompilerConfiguration,
        schedule: Schedule,
    ) -> Self {
        let units = schedule.units.len();
        Self {
            collection,
            configuration,
            queue: Mutex::new(Queue {
                builds: (0..units).collect(),
                runs: VecDeque::new(),
                building: 0,
            }),
            changed: Condvar::new(),
            cancelled: AtomicBool::new(false),
            executables: (0..units).map(|_| OnceLock::new()).collect(),
            remaining: (0..units).map(|_| AtomicUsize::new(0)).collect(),
            schedule,
        }
    }

    fn work(&self, sender: &Sender<(usize, TestResult, Duration)>) {
        while let Some(job) = self.next_job() {
            match job {
                Job::Build(unit) => self.build(unit, sender),
                Job::Run { unit, position } => {
                    let started = Instant::now();
                    let result = self.run(unit, position);
                    let flat = self.schedule.flat[self.schedule.units[unit].group][position];
                    sender
                        .send((flat, result, started.elapsed()))
                        .expect("result receiver dropped");
                }
            }
        }
    }

    /// Stops the workers from taking further jobs. The queue lock is held while
    /// the flag is set so that no worker misses the wake-up.
    fn cancel(&self) {
        let _queue = self.queue.lock().expect("test queue poisoned");
        self.cancelled.store(true, Ordering::SeqCst);
        self.changed.notify_all();
    }

    /// The next job, waiting while a running build may still queue tests;
    /// `None` when all work is done.
    fn next_job(&self) -> Option<Job> {
        let mut queue = self.queue.lock().expect("test queue poisoned");
        loop {
            if self.cancelled.load(Ordering::SeqCst) {
                return None;
            }
            if let Some((unit, position)) = queue.runs.pop_front() {
                return Some(Job::Run { unit, position });
            }
            if let Some(unit) = queue.builds.pop_front() {
                queue.building += 1;
                return Some(Job::Build(unit));
            }
            if queue.building == 0 {
                return None;
            }
            queue = self.changed.wait(queue).expect("test queue poisoned");
        }
    }

    /// Builds one executable and queues its tests. A build that fails reports
    /// its diagnostic once, with the first test of the unit in canonical
    /// order, and fails every other test of the unit with a pointer to it.
    fn build(&self, unit: usize, sender: &Sender<(usize, TestResult, Duration)>) {
        let started = Instant::now();
        let build_unit = &self.schedule.units[unit];
        let group = &self.collection.groups[build_unit.group];
        let tests = build_unit
            .positions
            .iter()
            .map(|position| (*position, &group.tests[*position]))
            .collect::<Vec<_>>();
        // A panic while building must not leave the other workers waiting for
        // a build that never finishes.
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_executable(group, &tests, build_unit.dispatch, unit, self.configuration)
        }))
        .unwrap_or_else(|_| {
            Err("internal error: building the test executable panicked".to_owned())
        });
        let mut queue = self.queue.lock().expect("test queue poisoned");
        queue.building -= 1;
        match built {
            Ok(executable) => {
                self.remaining[unit].store(tests.len(), Ordering::SeqCst);
                let _ = self.executables[unit].set(executable);
                let mut ordered = build_unit.positions.clone();
                ordered.sort_by_key(|position| self.schedule.flat[build_unit.group][*position]);
                queue.runs.extend(ordered.into_iter().map(|position| (unit, position)));
            }
            Err(error) => {
                let mut ordered = build_unit.positions.clone();
                ordered.sort_by_key(|position| self.schedule.flat[build_unit.group][*position]);
                let first = &group.tests[ordered[0]];
                for (index, position) in ordered.iter().enumerate() {
                    let message = if index == 0 {
                        error.clone()
                    } else {
                        format!(
                            "test group `{}` failed to build; the diagnostic is reported with `{}::{}`",
                            group.label, first.display_module, first.name
                        )
                    };
                    let flat = self.schedule.flat[build_unit.group][*position];
                    let _ = sender.send((flat, Err(message), started.elapsed()));
                }
            }
        }
        drop(queue);
        self.changed.notify_all();
    }

    /// Runs one test of a built executable in its own process, working
    /// directory and timeout. The last test of a unit removes the executable.
    fn run(&self, unit: usize, position: usize) -> TestResult {
        let executable =
            self.executables[unit].get().expect("executable built before its tests run");
        let selection = self.schedule.units[unit].dispatch.then_some(position);
        let process = executable.working_directory(position).and_then(|directory| {
            let result = run_test_process(&executable.executable, &directory, selection);
            combine(result, remove_artifacts(&directory))
        });
        let finished = self.remaining[unit].fetch_sub(1, Ordering::SeqCst) == 1;
        combine(process, if finished { remove_artifacts(&executable.directory) } else { Ok(()) })
    }
}

/// A test result that is also an error when its cleanup failed.
fn combine(result: TestResult, cleanup: Result<(), String>) -> TestResult {
    match (result, cleanup) {
        (Ok(process), Ok(())) => Ok(process),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(run_error), Err(cleanup_error)) => Err(format!("{run_error}; {cleanup_error}")),
    }
}

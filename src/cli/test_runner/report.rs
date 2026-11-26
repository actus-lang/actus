//! The deterministic report of `actus test`: one line per test in canonical
//! order and a summary.

use crate::ast::TestFault;
use crate::cli::terminal_output::{OutputStopped, try_outln};

use super::super::conformance::ConformanceMode;
use super::collection::DiscoveredTest;
use super::output::TestOutputStyle;
use super::process::{TestProcessResult, append_stderr_for_report};

pub(super) fn print_test_header(
    total: usize,
    filtered: usize,
    mode: ConformanceMode,
) -> Result<(), OutputStopped> {
    let noun = if total == 1 { "test" } else { "tests" };
    if mode.is_strict() {
        try_outln!("running {total} {noun} (strict) ({filtered} filtered)")
    } else {
        try_outln!("running {total} {noun} ({filtered} filtered)")
    }
}

pub(super) fn report_test_result(
    test: &DiscoveredTest,
    result: Result<TestProcessResult, String>,
    elapsed: std::time::Duration,
    output_style: &TestOutputStyle,
) -> Result<bool, OutputStopped> {
    let elapsed = elapsed.as_millis();
    let label = format!("test {}::{}", test.display_module, test.name);
    let (status, passed) = match result {
        Ok(TestProcessResult::Exited { code: 0, .. }) => {
            (output_style.success(&format!("ok ({elapsed} ms, exit code 0)")), true)
        }
        Ok(TestProcessResult::Exited { code, stderr }) => {
            let mut detail = format!("FAILED ({elapsed} ms, exit code {code})");
            append_stderr_for_report(&mut detail, &stderr);
            (output_style.failure(&detail), false)
        }
        Ok(TestProcessResult::Signaled { .. })
            if matches!(
                test.fault,
                Some(TestFault::InterruptedBeforeRename | TestFault::InterruptedPublication)
            ) =>
        {
            (output_style.success(&format!("ok ({elapsed} ms, expected interruption)")), true)
        }
        Ok(TestProcessResult::Signaled { signal, stderr }) => {
            let signal = signal.map_or_else(|| "unknown".to_owned(), |value| value.to_string());
            let mut detail = format!("FAILED ({elapsed} ms, terminated by signal {signal})");
            append_stderr_for_report(&mut detail, &stderr);
            (output_style.failure(&detail), false)
        }
        Err(error) => (output_style.failure(&format!("FAILED ({elapsed} ms, {error})")), false),
    };
    try_outln!("{label} ... {status}")?;
    Ok(passed)
}

pub(super) fn print_test_summary(
    total: usize,
    passed: usize,
    elapsed: std::time::Duration,
    output_style: &TestOutputStyle,
) -> Result<i32, OutputStopped> {
    let failed = total - passed;
    let result =
        if failed == 0 { output_style.success("ok") } else { output_style.failure("FAILED") };
    try_outln!(
        "test result: {result}. {passed} passed; {failed} failed; finished in {}",
        format_duration(elapsed)
    )?;
    Ok(i32::from(failed != 0))
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

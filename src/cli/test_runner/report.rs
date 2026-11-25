//! The deterministic report of `actus test`: one line per test in canonical
//! order and a summary.

use crate::ast::TestFault;

use super::super::conformance::ConformanceMode;
use super::collection::DiscoveredTest;
use super::output::TestOutputStyle;
use super::process::{TestProcessResult, append_stderr_for_report};

pub(super) fn print_test_header(total: usize, filtered: usize, mode: ConformanceMode) {
    if mode.is_strict() {
        println!("running {total} tests (strict) ({filtered} filtered)");
    } else {
        println!("running {total} tests ({filtered} filtered)");
    }
}

pub(super) fn report_test_result(
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
            append_stderr_for_report(&mut detail, &stderr);
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
            append_stderr_for_report(&mut detail, &stderr);
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

pub(super) fn print_test_summary(
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

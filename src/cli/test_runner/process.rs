use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_TEST_PROCESS_TIMEOUT_SECONDS: u64 = 30;
const TEST_PROCESS_TIMEOUT_ENV: &str = "ACTUS_TEST_TIMEOUT_SECONDS";

#[derive(Debug, Eq, PartialEq)]
pub(super) enum TestProcessResult {
    Exited(i32),
    Signaled,
}

pub(super) fn run_test_process(
    executable: &Path,
    working_directory: &Path,
) -> Result<TestProcessResult, String> {
    let timeout = configured_test_process_timeout()?;
    let mut child = Command::new(executable)
        .current_dir(working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Ok(process_result(status));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("timed out after {} seconds", timeout.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn configured_test_process_timeout() -> Result<Duration, String> {
    let value = std::env::var(TEST_PROCESS_TIMEOUT_ENV).ok();
    parse_test_process_timeout(value.as_deref())
}

fn parse_test_process_timeout(value: Option<&str>) -> Result<Duration, String> {
    let seconds = match value {
        None => DEFAULT_TEST_PROCESS_TIMEOUT_SECONDS,
        Some(raw) => raw.parse::<u64>().map_err(|_| {
            format!("{TEST_PROCESS_TIMEOUT_ENV} must be a positive integer number of seconds")
        })?,
    };
    if seconds == 0 {
        return Err(format!(
            "{TEST_PROCESS_TIMEOUT_ENV} must be a positive integer number of seconds"
        ));
    }
    Ok(Duration::from_secs(seconds))
}

fn process_result(status: std::process::ExitStatus) -> TestProcessResult {
    if let Some(code) = status.code() {
        return TestProcessResult::Exited(code);
    }
    TestProcessResult::Signaled
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        DEFAULT_TEST_PROCESS_TIMEOUT_SECONDS, TEST_PROCESS_TIMEOUT_ENV, parse_test_process_timeout,
    };

    #[test]
    fn default_timeout_is_thirty_seconds() {
        assert_eq!(DEFAULT_TEST_PROCESS_TIMEOUT_SECONDS, 30);
    }

    #[test]
    fn timeout_environment_name_is_stable() {
        assert_eq!(TEST_PROCESS_TIMEOUT_ENV, "ACTUS_TEST_TIMEOUT_SECONDS");
    }

    #[test]
    fn timeout_configuration_accepts_positive_integer() {
        assert_eq!(parse_test_process_timeout(Some("60")), Ok(Duration::from_secs(60)));
    }

    #[test]
    fn timeout_configuration_rejects_zero() {
        assert!(parse_test_process_timeout(Some("0")).is_err());
    }

    #[test]
    fn timeout_configuration_rejects_non_integer() {
        assert!(parse_test_process_timeout(Some("slow")).is_err());
    }
}

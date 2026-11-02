use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_TEST_PROCESS_TIMEOUT_SECONDS: u64 = 30;
const TEST_PROCESS_TIMEOUT_ENV: &str = "ACTUS_TEST_TIMEOUT_SECONDS";
const MAX_TEST_DIAGNOSTIC_BYTES: usize = 64 * 1024;

#[derive(Debug, Eq, PartialEq)]
pub(super) enum TestProcessResult {
    Exited { code: i32, stderr: String },
    Signaled { signal: Option<i32>, stderr: String },
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
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "test process stderr pipe was not available".to_owned())?;
    let stderr_reader = thread::spawn(move || read_bounded_stderr(&mut stderr));
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            let stderr = stderr_reader
                .join()
                .map_err(|_| "test process stderr reader panicked".to_owned())??;
            return Ok(process_result(status, stderr));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let stderr = stderr_reader
                .join()
                .map_err(|_| "test process stderr reader panicked".to_owned())??;
            return Err(format_timeout(timeout, &stderr));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn read_bounded_stderr(stderr: &mut impl Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    stderr
        .take((MAX_TEST_DIAGNOSTIC_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let truncated = bytes.len() > MAX_TEST_DIAGNOSTIC_BYTES;
    bytes.truncate(MAX_TEST_DIAGNOSTIC_BYTES);
    let mut diagnostic = String::from_utf8_lossy(&bytes).into_owned();
    if truncated {
        diagnostic.push_str("\n[test stderr truncated after 65536 bytes]");
    }
    Ok(diagnostic)
}

fn format_timeout(timeout: Duration, stderr: &str) -> String {
    let mut message = format!("timed out after {} seconds", timeout.as_secs());
    append_stderr(&mut message, stderr);
    message
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

fn process_result(status: std::process::ExitStatus, stderr: String) -> TestProcessResult {
    if let Some(code) = status.code() {
        return TestProcessResult::Exited { code, stderr };
    }
    TestProcessResult::Signaled { signal: signal_number(status), stderr }
}

#[cfg(unix)]
fn signal_number(status: std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;

    status.signal()
}

#[cfg(not(unix))]
fn signal_number(_status: std::process::ExitStatus) -> Option<i32> {
    None
}

fn append_stderr(message: &mut String, stderr: &str) {
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        message.push_str(&format!("; stderr=```\n{stderr}\n```"));
    }
}

pub(super) fn append_stderr_for_report(message: &mut String, stderr: &str) {
    append_stderr(message, stderr);
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

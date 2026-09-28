use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TEST_PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Eq, PartialEq)]
pub(super) enum TestProcessResult {
    Exited(i32),
    Signaled,
}

pub(super) fn run_test_process(executable: &Path) -> Result<TestProcessResult, String> {
    let mut child = Command::new(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + TEST_PROCESS_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Ok(process_result(status));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("timed out after {} seconds", TEST_PROCESS_TIMEOUT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn process_result(status: std::process::ExitStatus) -> TestProcessResult {
    if let Some(code) = status.code() {
        return TestProcessResult::Exited(code);
    }
    TestProcessResult::Signaled
}

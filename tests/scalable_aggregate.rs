#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CASES: &[(&str, &str)] =
    &[("64", "aggregate_64.act"), ("1024", "aggregate_1024.act"), ("65536", "aggregate_65536.act")];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scalable_aggregate")
        .join(name)
}

fn run_cli(arguments: &[String]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .output()
        .expect("run Actus compiler child process")
}

fn status_label(status: std::process::ExitStatus) -> String {
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    if let Some(code) = status.code() {
        return format!("exit={code}");
    }
    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return format!("signal={signal} shell_status={}", 128 + signal);
    }
    "unknown_status".to_owned()
}

fn run_case(size: &str, file: &str) {
    let input = fixture(file);
    let root = std::env::temp_dir().join(format!("actus-scale-{size}-{}", std::process::id()));
    let output = root.with_extension("bin");
    let check = run_cli(&["check".to_owned(), input.display().to_string(), "--strict".to_owned()]);
    println!("SCALE_CASE size={size} stage=check {}", status_label(check.status));
    assert!(check.status.success(), "check failed for {size}: {:?}", check.stderr);

    let build = run_cli(&[
        "build".to_owned(),
        input.display().to_string(),
        "--strict".to_owned(),
        "--emit".to_owned(),
        "exe".to_owned(),
        "-o".to_owned(),
        output.display().to_string(),
    ]);
    println!("SCALE_CASE size={size} stage=build {}", status_label(build.status));
    assert!(build.status.success(), "build failed for {size}: {:?}", build.stderr);

    let metadata = fs::metadata(&output).expect("read executable metadata");
    println!("SCALE_CASE size={size} executable_bytes={}", metadata.len());
    let execution = Command::new(&output).output().expect("run scale executable");
    println!("SCALE_CASE size={size} stage=run {}", status_label(execution.status));
    assert_eq!(execution.status.code(), Some(42));
    let _ = fs::remove_file(output);
}

#[test]
fn compiles_and_runs_small_scalable_aggregate_fixtures() {
    for (size, file) in CASES.iter() {
        run_case(size, file);
    }
}

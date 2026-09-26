use std::fs;
use std::process::Command;

use actus::cli::run_with_args;

#[cfg(unix)]
#[test]
fn invokes_user_drop_contract_in_lifo_order() {
    let root = std::env::temp_dir().join(format!("actus-drop-native-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { print(self.value); } } verb main() -> Int { { erg first = Counter { value: 1, }; erg second = Counter { value: 2, }; } return 0; }";
    fs::write(&input, source).expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let execution = Command::new(&output).output().expect("run executable");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&execution.stdout), "2\n1\n");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn try_error_path_runs_drop_before_returning_err() {
    let root = std::env::temp_dir().join(format!("actus-drop-try-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { print(self.value); } } verb fail() -> Result[Int, Int] { return Err(1); } verb worker() -> Result[Int, Int] { erg counter = Counter { value: 9, }; fail()?; return Ok(0); } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(_) => 1, Result.Err(_) => 0, }; }";
    fs::write(&input, source).expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let execution = Command::new(&output).output().expect("run executable");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&execution.stdout), "9\n");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

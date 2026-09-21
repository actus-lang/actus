use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::{EmitKind, build_file};

pub(super) fn run_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let Some(input) = arguments.next() else {
        eprintln!("error: missing input file");
        return 2;
    };
    if arguments.next().is_some() {
        eprintln!("error: unexpected extra argument");
        return 2;
    }

    let output = temporary_output();
    if build_file(&input, Some(&output), EmitKind::Executable) != 0 {
        return 1;
    }
    let result = Command::new(&output).status();
    let _ = fs::remove_file(&output);
    match result {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("error: cannot run `{}`: {error}", output.display());
            1
        }
    }
}

fn temporary_output() -> PathBuf {
    std::env::temp_dir().join(format!("actus-run-{}", std::process::id()))
}

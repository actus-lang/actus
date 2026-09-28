use std::fs;
use std::path::Path;

use crate::configuration::{ActusLock, LockfileError};

pub(super) fn lock_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let check_only = match parse_options(&mut arguments) {
        Ok(check_only) => check_only,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let Some(manifest) = crate::configuration::manifest_path_in_directory(Path::new(".")) else {
        eprintln!("error: cannot find `Actus.toml` in the current project");
        return 1;
    };
    if check_only {
        return check_lockfile(&manifest);
    }
    match ActusLock::sync(&manifest) {
        Ok(path) => {
            println!("updated `{}`", path.display());
            0
        }
        Err(error) => report_lock_error(error),
    }
}

fn parse_options(arguments: &mut impl Iterator<Item = String>) -> Result<bool, String> {
    let mut check_only = false;
    for argument in arguments {
        if argument != "--check" {
            return Err(format!("unexpected lock argument `{argument}`"));
        }
        if check_only {
            return Err("duplicate `--check` option".to_owned());
        }
        check_only = true;
    }
    Ok(check_only)
}

fn check_lockfile(manifest: &Path) -> i32 {
    let path = manifest.with_file_name("Actus.lock");
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{}`: {error}", path.display());
            return 1;
        }
    };
    match ActusLock::parse(&source).and_then(|lock| lock.validate_against_manifest(manifest)) {
        Ok(()) => {
            println!("checked `{}` successfully", path.display());
            0
        }
        Err(error) => report_lock_error(error),
    }
}

fn report_lock_error(error: LockfileError) -> i32 {
    eprintln!("error: {error}");
    1
}

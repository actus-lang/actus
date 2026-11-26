//! Builds the blocks of one entry section into a project and runs `actus` on it.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::entry::{Block, is_code};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

/// The blocks of one section that are built together. A block whose file is
/// already in the group starts the next group, so several examples of one
/// section are separate projects.
pub fn groups<'a>(blocks: &[&'a Block]) -> Vec<Vec<&'a Block>> {
    let mut groups: Vec<Vec<&'a Block>> = Vec::new();
    for &block in blocks {
        let starts_group = groups.last().is_none_or(|group| {
            group.first().is_some_and(|first| first.section != block.section)
                || group.iter().any(|member| member.target() == block.target())
        });
        if starts_group {
            groups.push(Vec::new());
        }
        groups.last_mut().expect("a group was just pushed").push(block);
    }
    groups
}

/// A project in a temporary directory that is removed when it is dropped.
pub struct TempProject {
    base: PathBuf,
    root: PathBuf,
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// What `actus` printed for one run.
pub struct Run {
    pub success: bool,
    pub output: String,
}

impl TempProject {
    /// Runs `actus new`, then writes the blocks over the template.
    pub fn create(group: &[&Block]) -> Result<Self, String> {
        let base = std::env::temp_dir().join(format!(
            "actus-diagnostic-catalog-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&base).map_err(|error| error.to_string())?;
        let project = TempProject { root: base.join("example"), base };
        let created = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["new", "example", "--no-git"])
            .current_dir(&project.base)
            .output()
            .map_err(|error| error.to_string())?;
        if !created.status.success() {
            return Err(format!(
                "`actus new` failed: {}",
                String::from_utf8_lossy(&created.stderr)
            ));
        }
        for block in group {
            let path = project.root.join(block.target());
            let parent = path.parent().expect("a block target has a parent directory");
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            let body = super::repeat::expand(&block.body)?;
            fs::write(&path, body).map_err(|error| error.to_string())?;
        }
        Ok(project)
    }

    /// Runs `actus <command>` (with `--strict` when `strict`) in the project
    /// and joins both output streams.
    pub fn run(&self, command: &str, strict: bool) -> Run {
        let mut process = Command::new(env!("CARGO_BIN_EXE_actus"));
        process.arg(command).current_dir(&self.root);
        if strict {
            process.arg("--strict");
        }
        let output = process.output().expect("the actus binary should start");
        let output_text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        Run { success: output.status.success(), output: output_text }
    }
}

/// The first error line of a run: its code and, when the line names a file,
/// the location in front of it. A line has the one format of every diagnostic,
/// `<location>: error[E####] ...`, or `error[E####]: ...` when it names no
/// file (an error of the toolchain or the environment).
pub struct FirstError {
    pub code: String,
    pub location: Option<String>,
}

pub fn first_error(run: &Run) -> Option<FirstError> {
    run.output.lines().find_map(|line| {
        let (location, rest) = match line.strip_prefix("error[") {
            Some(rest) => (None, rest),
            None => {
                let (location, rest) = line.split_once(": error[")?;
                (Some(location.replace('\\', "/")), rest)
            }
        };
        let code = rest.split(']').next().filter(|code| is_code(code))?;
        Some(FirstError { code: code.to_owned(), location })
    })
}

/// The codes of errors of the toolchain or the environment. They name no file,
/// so their first error line has no location to check (ADR-0123).
const ENVIRONMENT_CODES: [&str; 4] = ["E1803", "E1806", "E1880", "E1881"];

/// Whether the first error of a run is the one the entry of `code` promises:
/// its code, at a location inside the example. A line without a file is
/// accepted only for a code that is an environment code.
pub fn is_expected(error: &FirstError, code: &str, files: &[&str]) -> bool {
    if error.code != code {
        return false;
    }
    match &error.location {
        Some(location) => inside_example(location, files),
        None => ENVIRONMENT_CODES.contains(&code),
    }
}

/// Whether the reported `location` lies inside the example: it is one of the
/// example's files, or a directory that holds one. `actus test` names the
/// directory or file of a layout error, with an `error: ` prefix and a `./`.
pub fn inside_example(location: &str, files: &[&str]) -> bool {
    let location = location.strip_prefix("error: ").unwrap_or(location);
    let location = location.strip_prefix("./").unwrap_or(location);
    files.iter().any(|file| {
        location.ends_with(file)
            || file.starts_with(&format!("{}/", location.trim_end_matches('/')))
    })
}

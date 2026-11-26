//! Builds the blocks of one entry section into a project and runs `actus` on it.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::entry::Block;

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
            fs::write(&path, &block.body).map_err(|error| error.to_string())?;
        }
        Ok(project)
    }

    /// Runs `actus <command>` in the project and joins both output streams.
    pub fn run(&self, command: &str) -> Run {
        let output = Command::new(env!("CARGO_BIN_EXE_actus"))
            .arg(command)
            .current_dir(&self.root)
            .output()
            .expect("the actus binary should start");
        let output_text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        Run { success: output.status.success(), output: output_text }
    }
}

/// The code and location of the first `error[E####]` line of a run.
pub fn first_error(run: &Run) -> Option<(String, String)> {
    let line = run.output.lines().find(|line| line.contains(": error[E"))?;
    let (location, rest) = line.split_once(": error[")?;
    let code = rest.split(']').next()?;
    Some((code.to_owned(), location.replace('\\', "/")))
}

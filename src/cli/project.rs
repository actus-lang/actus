use crate::cli::terminal_output::{errln, outln};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::configuration::RuntimeProfile;

const MANIFEST: &str = "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\ntarget = \"host\"\nprofile = \"debug\"{runtime}\n";
const MAIN_SOURCE: &str = "verb main() {\n    print(string: \"Hello, world!\");\n}\n";
const GITIGNORE: &str = "/capsula/\nActus.lock\n*.o\n*.bin\n*.actus\n";

struct ProjectOptions {
    vcs: bool,
    runtime: RuntimeProfile,
    path: PathBuf,
}

pub(super) fn new_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let Some(name) = arguments.next() else {
        errln!("error: missing project name");
        return 2;
    };
    if super::input::is_option_like(&name) {
        errln!("error: {}", super::input::argument_error("new", &name));
        return 2;
    }
    let options = match parse_options(&name, arguments, true) {
        Ok(options) => options,
        Err(error) => {
            errln!("error: {error}");
            return 2;
        }
    };
    if options.path.exists() {
        errln!("error: project path `{}` already exists", options.path.display());
        return 1;
    }
    let package_name = Path::new(&name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(&name);
    if let Err(error) = create_project(&options.path, package_name, &options) {
        errln!("error: {error}");
        return 1;
    }
    outln!("created `{}`", options.path.display());
    print_next_steps(&options.path, true);
    0
}

pub(super) fn init_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_options("project", arguments.by_ref(), false) {
        Ok(options) => options,
        Err(error) => {
            errln!("error: {error}");
            return 2;
        }
    };
    if let Err(error) = initialize_project(&options.path, &options) {
        errln!("error: {error}");
        return 1;
    }
    outln!("initialized `{}`", options.path.display());
    print_next_steps(&options.path, false);
    0
}

fn print_next_steps(path: &Path, include_directory_change: bool) {
    outln!("");
    outln!("Next steps:");
    if include_directory_change {
        outln!("  cd {}", path.display());
    }
    outln!("  actus check");
    outln!("  actus build --emit exe");
    outln!("  actus build --release --emit exe");
    outln!("  actus run");
}

fn parse_options(
    name: &str,
    mut arguments: impl Iterator<Item = String>,
    create_path: bool,
) -> Result<ProjectOptions, String> {
    let mut vcs = true;
    // New projects use the hosted standard library; `--runtime core` opts out.
    let mut runtime = RuntimeProfile::Std;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--no-git" => vcs = false,
            "--vcs" => match arguments.next().as_deref() {
                Some("none") => vcs = false,
                Some("git") => vcs = true,
                Some(value) => return Err(format!("unsupported VCS `{value}`")),
                None => return Err("missing value after `--vcs`".to_owned()),
            },
            "--runtime" => {
                runtime = parse_runtime(arguments.next().as_deref())?;
            }
            _ => {
                let command = if create_path { "new" } else { "init" };
                return Err(super::input::argument_error(command, &argument));
            }
        }
    }
    let path = if create_path {
        PathBuf::from(name)
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot read current directory: {error}"))?
    };
    Ok(ProjectOptions { vcs, runtime, path })
}

fn parse_runtime(value: Option<&str>) -> Result<RuntimeProfile, String> {
    match value {
        Some("core") => Ok(RuntimeProfile::Core),
        Some("std") => Ok(RuntimeProfile::Std),
        Some("freestanding") => Err(
            "`--runtime freestanding` requires an explicit target; use the target-aware project workflow"
                .to_owned(),
        ),
        Some(value) => Err(format!("unsupported runtime profile `{value}`")),
        None => Err("missing value after `--runtime`".to_owned()),
    }
}

fn create_project(path: &Path, name: &str, options: &ProjectOptions) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("cannot create project: {error}"))?;
    initialize_project_files(path, name, options.runtime)?;
    initialize_vcs(path, options.vcs)
}

fn initialize_project(path: &Path, options: &ProjectOptions) -> Result<(), String> {
    if path.join("Actus.toml").exists() {
        return Err(format!("`{}` is already an Actus project", path.display()));
    }
    initialize_project_files(path, "project", options.runtime)?;
    initialize_vcs(path, options.vcs)
}

fn initialize_project_files(
    path: &Path,
    name: &str,
    runtime: RuntimeProfile,
) -> Result<(), String> {
    let source = path.join("src");
    fs::create_dir_all(&source).map_err(|error| format!("cannot create src/: {error}"))?;
    fs::write(path.join("Actus.toml"), manifest_for(name, runtime))
        .map_err(|error| format!("cannot write Actus.toml: {error}"))?;
    fs::write(source.join("main.act"), MAIN_SOURCE)
        .map_err(|error| format!("cannot write src/main.act: {error}"))?;
    fs::write(path.join(".gitignore"), GITIGNORE)
        .map_err(|error| format!("cannot write .gitignore: {error}"))
}

fn manifest_for(name: &str, runtime: RuntimeProfile) -> String {
    let runtime = match runtime {
        RuntimeProfile::Core => String::new(),
        RuntimeProfile::Std => "\nruntime = \"std\"".to_owned(),
        RuntimeProfile::Freestanding => unreachable!("freestanding templates are rejected"),
    };
    MANIFEST.replace("{name}", name).replace("{runtime}", &runtime)
}

fn initialize_vcs(path: &Path, requested: bool) -> Result<(), String> {
    if !requested || has_git_ancestor(path) {
        return Ok(());
    }
    let status = Command::new("git")
        .args(["-c", "init.defaultBranch=main", "init"])
        .arg(path)
        .status()
        .map_err(|error| format!("cannot initialize git: {error}"))?;
    if status.success() { Ok(()) } else { Err("git init failed".to_owned()) }
}

fn has_git_ancestor(path: &Path) -> bool {
    path.ancestors().any(|directory| directory.join(".git").exists())
}

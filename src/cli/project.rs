use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MANIFEST: &str = "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\ntarget = \"host\"\nprofile = \"debug\"\n";
const MAIN_SOURCE: &str = "verb main() -> Int { return 0; }\n";
const GITIGNORE: &str = "/capsula/\n*.o\n*.bin\n";

pub(super) fn new_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let Some(name) = arguments.next() else {
        eprintln!("error: missing project name");
        return 2;
    };
    let (vcs, path) = match parse_options(&name, arguments, true) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    if path.exists() {
        eprintln!("error: project path `{}` already exists", path.display());
        return 1;
    }
    let package_name = Path::new(&name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(&name);
    if let Err(error) = create_project(&path, package_name, vcs) {
        eprintln!("error: {error}");
        return 1;
    }
    println!("created `{}`", path.display());
    0
}

pub(super) fn init_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let (vcs, path) = match parse_options("project", arguments.by_ref(), false) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    if let Err(error) = initialize_project(&path, vcs) {
        eprintln!("error: {error}");
        return 1;
    }
    println!("initialized `{}`", path.display());
    0
}

fn parse_options(
    name: &str,
    mut arguments: impl Iterator<Item = String>,
    create_path: bool,
) -> Result<(bool, PathBuf), String> {
    let mut vcs = true;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--no-git" => vcs = false,
            "--vcs" => match arguments.next().as_deref() {
                Some("none") => vcs = false,
                Some("git") => vcs = true,
                Some(value) => return Err(format!("unsupported VCS `{value}`")),
                None => return Err("missing value after `--vcs`".to_owned()),
            },
            _ => return Err(format!("unexpected argument `{argument}`")),
        }
    }
    let path = if create_path {
        PathBuf::from(name)
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot read current directory: {error}"))?
    };
    Ok((vcs, path))
}

fn create_project(path: &Path, name: &str, vcs: bool) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("cannot create project: {error}"))?;
    initialize_project_files(path, name)?;
    initialize_vcs(path, vcs)
}

fn initialize_project(path: &Path, vcs: bool) -> Result<(), String> {
    if path.join("Arca.toml").exists() {
        return Err(format!("`{}` is already an Actus project", path.display()));
    }
    initialize_project_files(path, "project")?;
    initialize_vcs(path, vcs)
}

fn initialize_project_files(path: &Path, name: &str) -> Result<(), String> {
    let source = path.join("src");
    fs::create_dir_all(&source).map_err(|error| format!("cannot create src/: {error}"))?;
    fs::write(path.join("Arca.toml"), MANIFEST.replace("{name}", name))
        .map_err(|error| format!("cannot write Arca.toml: {error}"))?;
    fs::write(source.join("main.act"), MAIN_SOURCE)
        .map_err(|error| format!("cannot write src/main.act: {error}"))?;
    fs::write(path.join(".gitignore"), GITIGNORE)
        .map_err(|error| format!("cannot write .gitignore: {error}"))
}

fn initialize_vcs(path: &Path, requested: bool) -> Result<(), String> {
    if !requested || has_git_ancestor(path) {
        return Ok(());
    }
    let status = Command::new("git")
        .arg("init")
        .arg(path)
        .status()
        .map_err(|error| format!("cannot initialize git: {error}"))?;
    if status.success() { Ok(()) } else { Err("git init failed".to_owned()) }
}

fn has_git_ancestor(path: &Path) -> bool {
    path.ancestors().any(|directory| directory.join(".git").exists())
}

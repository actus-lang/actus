use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, SystemTime};

use super::build::{EmitKind, build_file};
use super::check::check_command;
use crate::configuration::{BuildProfile, CompilerConfiguration};

struct WatchOptions {
    input: String,
    build: bool,
    once: bool,
    interval: Duration,
    profile: Option<BuildProfile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileStamp {
    modified: Option<SystemTime>,
    length: u64,
}

type FileSnapshot = BTreeMap<PathBuf, FileStamp>;

pub(super) fn watch_command(arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_watch_options(arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let configuration = match CompilerConfiguration::from_input_path(Path::new(&options.input)) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let configuration = options
        .profile
        .map_or(configuration.clone(), |profile| configuration.with_profile(profile));
    if options.once {
        println!("watching `{}`", options.input);
        return execute_once(&options, &configuration);
    }
    let mut snapshot = snapshot_paths(&watched_paths(&configuration, &options.input));
    let interrupted = match install_interrupt_handler() {
        Ok(interrupted) => interrupted,
        Err(error) => {
            eprintln!("error: cannot install watch interrupt handler: {error}");
            return 1;
        }
    };
    println!("watching `{}`", options.input);
    execute_once(&options, &configuration);
    while !interrupted.load(Ordering::Relaxed) {
        thread::sleep(options.interval);
        if interrupted.load(Ordering::Relaxed) {
            break;
        }
        let current = snapshot_paths(&watched_paths(&configuration, &options.input));
        if current == snapshot {
            continue;
        }
        snapshot = current;
        clear_refresh_output();
        println!("change detected; checking `{}`", options.input);
        let refresh_status = execute_once(&options, &configuration);
        if refresh_status == 0 {
            println!("watch check passed");
        }
    }
    eprintln!("watch interrupted");
    130
}

fn install_interrupt_handler() -> Result<Arc<AtomicBool>, String> {
    let interrupted = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&interrupted);
    ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed))
        .map_err(|error| error.to_string())?;
    Ok(interrupted)
}

fn clear_refresh_output() {
    print!("\x1b[2J\x1b[H");
}

fn parse_watch_options(
    mut arguments: impl Iterator<Item = String>,
) -> Result<WatchOptions, String> {
    let mut input = None;
    let mut build = false;
    let mut once = false;
    let mut interval = Duration::from_millis(200);
    let mut profile = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--build" if !build => build = true,
            "--once" if !once => once = true,
            "--release" => profile = replace_profile(profile, BuildProfile::Release)?,
            "--profile" => {
                let name = arguments.next().ok_or("missing value after `--profile`".to_owned())?;
                let selected = parse_profile(&name)?;
                profile = replace_profile(profile, selected)?;
            }
            "--interval" => {
                let value =
                    arguments.next().ok_or("missing value after `--interval`".to_owned())?;
                let milliseconds = value
                    .parse::<u64>()
                    .map_err(|_| "watch interval must be an integer in milliseconds".to_owned())?;
                if milliseconds == 0 {
                    return Err("watch interval must be greater than zero".to_owned());
                }
                interval = Duration::from_millis(milliseconds);
            }
            _ if input.is_none() && !argument.starts_with('-') => input = Some(argument),
            _ => return Err(format!("unexpected watch argument `{argument}`")),
        }
    }
    Ok(WatchOptions {
        input: input.unwrap_or_else(|| "src/main.act".to_owned()),
        build,
        once,
        interval,
        profile,
    })
}

fn parse_profile(name: &str) -> Result<BuildProfile, String> {
    match name {
        "debug" => Ok(BuildProfile::Debug),
        "release" => Ok(BuildProfile::Release),
        _ => Err(format!("unsupported build profile `{name}`")),
    }
}

fn replace_profile(
    current: Option<BuildProfile>,
    selected: BuildProfile,
) -> Result<Option<BuildProfile>, String> {
    if current.is_some() {
        return Err("duplicate or conflicting profile option".to_owned());
    }
    Ok(Some(selected))
}

fn execute_once(options: &WatchOptions, configuration: &CompilerConfiguration) -> i32 {
    if options.build {
        return build_file(&options.input, None, EmitKind::Executable, configuration);
    }
    check_command(std::iter::once(options.input.clone()))
}

fn watched_paths(configuration: &CompilerConfiguration, input: &str) -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(input), configuration.project_root().join("Arca.toml")];
    collect_act_files(configuration.source_root(), &mut paths);
    for source_root in configuration.dependency_roots().values() {
        collect_act_files(source_root, &mut paths);
    }
    paths.sort();
    paths.dedup();
    paths
}

fn collect_act_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    let mut entries = entries.filter_map(Result::ok).map(|entry| entry.path()).collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if !is_ignored_directory(&path) {
                collect_act_files(&path, paths);
            }
        } else if path.extension().is_some_and(|extension| extension == "act") {
            paths.push(path);
        }
    }
}

fn is_ignored_directory(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
        matches!(name, ".git" | "capsula" | "fixtures" | "snapshots" | "target")
    })
}

fn snapshot_paths(paths: &[PathBuf]) -> FileSnapshot {
    paths
        .iter()
        .filter_map(|path| {
            let metadata = fs::metadata(path).ok()?;
            Some((
                path.clone(),
                FileStamp { modified: metadata.modified().ok(), length: metadata.len() },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{is_ignored_directory, snapshot_paths};
    use std::fs;

    #[test]
    fn snapshots_detect_created_modified_and_removed_files() {
        let root = std::env::temp_dir().join(format!("actus-watch-state-{}", std::process::id()));
        fs::create_dir_all(&root).expect("create watch directory");
        let source = root.join("main.act");
        fs::write(&source, "verb main() -> Int { return 0; }").expect("write source");
        let initial = snapshot_paths(std::slice::from_ref(&source));
        fs::write(&source, "verb main() -> Int { return 1; }").expect("modify source");
        let modified = snapshot_paths(std::slice::from_ref(&source));
        assert_ne!(initial, modified);
        fs::remove_file(&source).expect("remove source");
        let removed = snapshot_paths(std::slice::from_ref(&source));
        assert_ne!(modified, removed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn excludes_generated_and_fixture_directories() {
        assert!(is_ignored_directory(std::path::Path::new("target")));
        assert!(is_ignored_directory(std::path::Path::new("fixtures")));
        assert!(!is_ignored_directory(std::path::Path::new("src")));
    }
}

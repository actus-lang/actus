use std::fs;
use std::path::{Path, PathBuf};

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{render_lex_error, render_parse_error};
use crate::formatter::format_program;
use crate::lexer::scan;
use crate::parser::parse;

pub(super) fn fmt_command(arguments: impl Iterator<Item = String>) -> i32 {
    let mut check_only = false;
    let mut input = None;
    for argument in arguments {
        match argument.as_str() {
            "--check" if !check_only => check_only = true,
            _ if input.is_none() => input = Some(PathBuf::from(argument)),
            _ => {
                eprintln!("error: unexpected argument `{argument}`");
                return 2;
            }
        }
    }
    let paths = input.map_or_else(project_sources, |path| vec![path]);
    if paths.is_empty() {
        eprintln!("error: no Actus source files found");
        return 1;
    }
    paths.into_iter().map(|path| format_file(&path, check_only)).max().unwrap_or(1)
}

fn project_sources() -> Vec<PathBuf> {
    let configuration = match CompilerConfiguration::from_current_manifest() {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return Vec::new();
        }
    };
    let mut paths = Vec::new();
    collect_act_files(configuration.source_root(), &mut paths);
    collect_act_files(&configuration.project_root().join("tests"), &mut paths);
    paths.sort();
    paths
}

fn collect_act_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    let mut entries = entries.filter_map(Result::ok).map(|entry| entry.path()).collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            let excluded = path
                .file_name()
                .is_some_and(|name| matches!(name.to_str(), Some("fixtures" | "snapshots")));
            if !excluded {
                collect_act_files(&path, paths);
            }
        } else if path.extension().is_some_and(|extension| extension == "act") {
            paths.push(path);
        }
    }
}

fn format_file(path: &Path, check_only: bool) -> i32 {
    let display = path.display();
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{display}`: {error}");
            return 1;
        }
    };
    let (tokens, lex_errors) = scan(&source);
    if let Some(error) = lex_errors.first() {
        eprintln!("{display}: {}", render_lex_error(&source, error));
        return 1;
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{display}: {}", render_parse_error(&source, &error));
            return 1;
        }
    };
    let formatted = format_program(&program);
    if check_only {
        if formatted == source {
            println!("{display} is formatted");
            0
        } else {
            eprintln!("{display} is not formatted");
            1
        }
    } else {
        match fs::write(path, formatted) {
            Ok(()) => {
                println!("formatted `{display}`");
                0
            }
            Err(error) => {
                eprintln!("error: cannot write `{display}`: {error}");
                1
            }
        }
    }
}

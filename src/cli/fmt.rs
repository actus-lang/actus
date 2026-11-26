use crate::cli::terminal_output::{errln, outln};
use std::fs;
use std::path::{Path, PathBuf};

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{render_lex_error, render_parse_error};
use crate::formatter::{SourceComment, format_program_with_comments, source_comments};
use crate::lexer::scan;
use crate::parser::parse;

pub(super) fn fmt_command(arguments: impl Iterator<Item = String>) -> i32 {
    let mut check_only = false;
    let mut input = None;
    for argument in arguments {
        match argument.as_str() {
            "--check" if !check_only => check_only = true,
            _ if input.is_none() && !super::input::is_option_like(&argument) => {
                input = Some(PathBuf::from(argument));
            }
            _ => {
                errln!("error: {}", super::input::argument_error("fmt", &argument));
                return 2;
            }
        }
    }
    let paths = input.map_or_else(project_sources, |path| vec![path]);
    if paths.is_empty() {
        errln!("error: no Actus source files found");
        return 1;
    }
    let project_wide = paths.len() > 1;
    let (mut code, mut written, mut unchanged) = (0, 0, 0);
    for path in paths {
        let (file_code, wrote) = format_file(&path, check_only);
        code = code.max(file_code);
        if file_code == 0 && !check_only {
            if wrote { written += 1 } else { unchanged += 1 }
        }
    }
    if project_wide && !check_only && code == 0 {
        let files = if written == 1 { "file" } else { "files" };
        outln!("formatted {written} {files}, {unchanged} already formatted");
    }
    code
}

fn project_sources() -> Vec<PathBuf> {
    let configuration = match CompilerConfiguration::from_current_manifest() {
        Ok(configuration) => configuration,
        Err(error) => {
            errln!("{}", error.report());
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

/// Returns the exit code and whether the file was rewritten.
fn format_file(path: &Path, check_only: bool) -> (i32, bool) {
    let display = path.display();
    let Some(source) = read_format_source(path) else { return (1, false) };
    let (tokens, lex_errors) = scan(&source);
    if let Some(error) = lex_errors.first() {
        errln!("{display}: {}", render_lex_error(&source, error));
        return (1, false);
    }
    let comments = source_comments(&source, &tokens);
    let Some(program) = parse_format_source(path, &source, tokens) else { return (1, false) };
    format_program_source(path, &source, &program, &comments, check_only)
}

fn read_format_source(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(source) => Some(source),
        Err(error) => {
            errln!("error: cannot read `{}`: {error}", path.display());
            None
        }
    }
}

fn parse_format_source(
    path: &Path,
    source: &str,
    tokens: Vec<crate::lexer::Token>,
) -> Option<crate::ast::Program> {
    match parse(tokens) {
        Ok(program) => Some(program),
        Err(error) => {
            errln!("{}: {}", path.display(), render_parse_error(source, &error));
            None
        }
    }
}

fn format_program_source(
    path: &Path,
    source: &str,
    program: &crate::ast::Program,
    comments: &[SourceComment],
    check_only: bool,
) -> (i32, bool) {
    let display = path.display();
    let formatted = format_program_with_comments(program, comments);
    if check_only {
        if formatted == source {
            outln!("{display} is formatted");
            (0, false)
        } else {
            errln!("{display} is not formatted");
            (1, false)
        }
    } else if formatted == source {
        outln!("`{display}` is already formatted");
        (0, false)
    } else {
        match fs::write(path, formatted) {
            Ok(()) => {
                outln!("formatted `{display}`");
                (0, true)
            }
            Err(error) => {
                errln!("error: cannot write `{display}`: {error}");
                (1, false)
            }
        }
    }
}

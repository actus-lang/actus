use std::fs;
use std::path::Path;

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
use crate::lexer::scan;
use crate::modules::{ModuleResolver, resolve_imports};
use crate::parser::parse;

pub(super) fn check_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let input = arguments.next().unwrap_or_else(|| "src/main.act".to_owned());
    if arguments.next().is_some() {
        eprintln!("error: unexpected extra argument");
        return 2;
    }
    let configuration = match CompilerConfiguration::from_input_path(Path::new(&input)) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let source = match fs::read_to_string(&input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{input}`: {error}");
            return 1;
        }
    };
    let (tokens, lex_errors) = scan(&source);
    if !lex_errors.is_empty() {
        for error in &lex_errors {
            eprintln!("{input}: {}", render_lex_error(&source, error));
        }
        return 1;
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{input}: {}", render_parse_error(&source, &error));
            return 1;
        }
    };
    let program = match resolve_imports(
        &program,
        &ModuleResolver::with_dependencies(
            configuration.source_root(),
            configuration.dependency_roots(),
        ),
    ) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("error: cannot resolve imports for `{input}`: {error}");
            return 1;
        }
    };
    match crate::semantic::analyze(&program) {
        Ok(_) => {
            println!("checked `{input}` successfully");
            0
        }
        Err(error) => {
            eprintln!("{input}: {}", render_semantic_error(&source, &error));
            1
        }
    }
}

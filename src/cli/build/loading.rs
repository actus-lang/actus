use std::fs;
use std::path::Path;

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{lex_diagnostic, module_diagnostic, parse_diagnostic, render_diagnostic};
use crate::lexer::scan;
use crate::modules::{ModuleResolver, resolve_imports};
use crate::parser::parse;

use super::super::diagnostics::{report_diagnostic, report_diagnostics};

pub(super) fn load_build_program(
    input: &str,
    configuration: &CompilerConfiguration,
) -> Option<(String, crate::ast::Program)> {
    let source = match fs::read_to_string(input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{input}`: {error}");
            return None;
        }
    };
    let (tokens, lex_errors) = scan(&source);
    if !lex_errors.is_empty() {
        report_diagnostics(
            Path::new(input),
            &source,
            lex_errors.iter().map(lex_diagnostic).collect(),
        );
        return None;
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            report_diagnostic(Path::new(input), &source, parse_diagnostic(&error));
            return None;
        }
    };
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    match resolve_imports(&program, &resolver) {
        Ok(program) => Some((source, program)),
        Err(error) => {
            let diagnostic = module_diagnostic(&error);
            eprintln!("{input}: {}", render_diagnostic(&source, &diagnostic));
            None
        }
    }
}

pub(super) fn validate_strict_program(
    input: &str,
    source: &str,
    program: &crate::ast::Program,
) -> bool {
    match crate::semantic::analyze(program) {
        Ok(_) => true,
        Err(error) => {
            report_diagnostic(
                Path::new(input),
                source,
                crate::diagnostics::semantic_diagnostic(&error),
            );
            false
        }
    }
}

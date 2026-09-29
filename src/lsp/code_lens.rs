use serde_json::{Value, json};

use crate::ast::TopLevelDecl;
use crate::configuration::CompilerConfiguration;
use crate::lexer::scan;
use crate::parser::parse;
use crate::semantic::filter_program_for_target;
use crate::target::TargetSpec;

use super::documents::DocumentStore;
use super::module_scope::file_uri_to_path;
use super::position::{LineIndex, LspRange};

const MAX_CODE_LENSES: usize = 16;

pub(super) fn entry_lenses(
    uri: &str,
    source: &str,
    version: i64,
    store: &DocumentStore,
    target: &TargetSpec,
) -> Value {
    let Some(path) = file_uri_to_path(uri) else { return Value::Array(Vec::new()) };
    let Some(configuration) = CompilerConfiguration::from_input_path_read_only(&path).ok() else {
        return Value::Array(Vec::new());
    };
    if configuration.target().triple() != target.triple() {
        return Value::Array(Vec::new());
    }
    let program = match parse(scan(source).0) {
        Ok(program) => filter_program_for_target(&program, target),
        Err(_) => return Value::Array(Vec::new()),
    };
    let Some(entry) = configuration.entry_symbol() else { return Value::Array(Vec::new()) };
    let lenses = program
        .declarations
        .iter()
        .filter_map(|declaration| entry_lens(declaration, entry, uri, source, version, store))
        .take(MAX_CODE_LENSES)
        .collect::<Vec<_>>();
    Value::Array(lenses)
}

fn entry_lens(
    declaration: &TopLevelDecl,
    entry: &str,
    uri: &str,
    source: &str,
    version: i64,
    store: &DocumentStore,
) -> Option<Value> {
    let TopLevelDecl::Verb(verb) = declaration else { return None };
    if verb.name != entry || store.get(uri)?.version != version {
        return None;
    }
    Some(json!({
        "range": declaration_range(source, verb.span),
        "command": {
            "title": "Run Actus entry point",
            "command": "actus.run",
            "arguments": [{"uri": uri, "version": version, "entry": entry}]
        }
    }))
}

fn declaration_range(source: &str, span: crate::lexer::SourceSpan) -> LspRange {
    let index = LineIndex::new(source);
    LspRange { start: index.position(source, span.start), end: index.position(source, span.end) }
}

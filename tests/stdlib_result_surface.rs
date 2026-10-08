use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use actus::ast::{PerformDecl, TopLevelDecl, VerbDecl};
use actus::conformance::source_paths;
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn public_fallible_standard_library_operations_have_typed_returns() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let files = source_paths(&[&root]).expect("standard-library sources should be discoverable");
    let infallible = infallible_verbs();
    for path in files {
        inspect_file(&path, &infallible);
    }
}

fn inspect_file(path: &Path, infallible: &BTreeSet<&'static str>) {
    let source = fs::read_to_string(path).expect("standard-library source should be readable");
    let program =
        parse(scan(&source).0).unwrap_or_else(|error| panic!("{}: {error:?}", path.display()));
    for declaration in program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) if verb.is_open => check_verb(path, &verb, infallible),
            TopLevelDecl::Perform(perform) if perform.is_open => {
                check_perform(path, &perform, infallible)
            }
            _ => {}
        }
    }
}

fn check_perform(path: &Path, perform: &PerformDecl, infallible: &BTreeSet<&'static str>) {
    for verb in &perform.methods {
        check_verb(path, verb, infallible);
    }
}

fn check_verb(path: &Path, verb: &VerbDecl, infallible: &BTreeSet<&'static str>) {
    if infallible.contains(verb.name.as_str()) {
        return;
    }
    let return_type = verb.return_type.as_ref().map(|return_type| return_type.ty.name.as_str());
    assert!(
        matches!(return_type, Some("Result" | "Option")),
        "{}: public fallible verb `{}` must return Result or Option",
        path.display(),
        verb.name
    );
}

fn infallible_verbs() -> BTreeSet<&'static str> {
    [
        "path_error_from_status",
        "windows_is_separator",
        "posix_is_separator",
        "is_absolute",
        "is_relative",
        "has_root",
        "starts_with",
        "ends_with",
        "components",
        "discard_path",
        "discard_handle",
        "options_new",
        "options_read",
        "options_write",
        "options_append",
        "options_truncate",
        "options_create",
        "options_create_new",
        "buffered_reader",
        "buffered_writer",
        "cursor",
        "drop",
        "monotonic_nanos",
        "now",
        "instant_ticks",
        "duration_nanos",
        "duration_as_nanos",
        "expired",
        "timer_one_shot",
        "timer_state",
        "wire_parser_empty",
        "wire_parser_reset",
        "wire_parser_status",
        "wire_parser_consumed",
        "wire_parser_header",
        "wire_reassembly_cancel",
        "wire_reassembly_open",
        "wire_sequence_empty",
        "wire_sequence_reset",
        "wire_sequence_replace_context",
        "wire_sequence_highest",
    ]
    .into_iter()
    .collect()
}

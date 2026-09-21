use std::fs;
use std::path::{Path, PathBuf};

use actus::diagnostics::render_semantic_error;
use actus::formatter::format_program;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("repository root should be readable"));
    update_parser_snapshot(&root);
    update_formatter_snapshot(&root);
    update_semantic_snapshot(&root);
    println!("updated compiler snapshots under `{}`", root.display());
}

fn update_parser_snapshot(root: &Path) {
    let fixture = root.join("tests/fixtures/parser/valid/transfer.act");
    let output = root.join("tests/fixtures/parser/snapshots/transfer.ast.snap");
    let program = parse_source(&fixture);
    write_snapshot(&output, &format!("{program:#?}"));
}

fn update_formatter_snapshot(root: &Path) {
    let fixture = root.join("tests/fixtures/formatter/valid/nested_blocks.act");
    let output = root.join("tests/fixtures/formatter/snapshots/nested_blocks.format.snap");
    let program = parse_source(&fixture);
    write_snapshot(&output, &format_program(&program));
}

fn update_semantic_snapshot(root: &Path) {
    let fixture = root.join("tests/fixtures/semantic/invalid/use_after_drop.act");
    let output = root.join("tests/fixtures/semantic/snapshots/use_after_drop.diag.snap");
    let source = fs::read_to_string(&fixture).expect("semantic fixture should be readable");
    let program = parse_source(&fixture);
    let error = analyze(&program).expect_err("semantic fixture should remain invalid");
    write_snapshot(&output, &render_semantic_error(&source, &error));
}

fn parse_source(path: &Path) -> actus::ast::Program {
    let source = fs::read_to_string(path).expect("snapshot fixture should be readable");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "snapshot fixture has lexer errors: {errors:?}");
    parse(tokens).expect("snapshot fixture should parse")
}

fn write_snapshot(path: &Path, contents: &str) {
    let contents = format!("{}\n", contents.trim_end());
    fs::write(path, contents).expect("snapshot should be writable");
}

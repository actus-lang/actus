use std::path::PathBuf;

use actus::lexer::scan;
use actus::modules::{ModuleResolver, analyze_module, exports_module};
use actus::parser::parse;
use actus::semantic::analyze;

#[test]
fn std_io_declarations_pass_semantic_validation() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let resolver = ModuleResolver::new(source_root);
    let exports = exports_module(&resolver, "io").expect("std io facade should resolve");
    for (kind, name) in [
        ("verb", "print_int"),
        ("verb", "eprint_int"),
        ("verb", "print"),
        ("verb", "println"),
        ("verb", "eprint"),
        ("verb", "eprintln"),
        ("verb", "flush"),
        ("verb", "read_line"),
        ("verb", "read_byte"),
        ("verb", "read"),
        ("verb", "write"),
        ("struct", "BufferedReader"),
        ("struct", "BufferedWriter"),
        ("verb", "buffered_reader"),
        ("verb", "buffered_writer"),
        ("verb", "reserve"),
        ("verb", "refill"),
        ("verb", "buffered_write"),
        ("verb", "flush_buffer"),
        ("struct", "Cursor"),
        ("verb", "cursor"),
        ("verb", "cursor_read"),
        ("verb", "cursor_write"),
        ("verb", "cursor_flush"),
        ("verb", "seek"),
        ("verb", "copy"),
        ("enum", "IoError"),
    ] {
        assert!(exports.contains(kind, name), "missing {kind} {name}");
    }
    analyze_module(&resolver, "io").expect("std io declarations should be semantically valid");
}

#[test]
fn std_io_buffered_constructor_requires_dat_ownership_transfer() {
    let source = "struct BufferedReader { erg buffer: Buffer, } verb buffered_reader(dat buffer: Buffer) -> BufferedReader { return BufferedReader { buffer: buffer, }; } verb main() -> Int { erg buffer = Buffer[4]; erg reader = buffered_reader(buffer: abs buffer); return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("buffered fixture should parse");
    assert!(analyze(&program).is_err());
}

#[test]
fn std_io_buffered_write_requires_distinct_ins_and_abs_bindings() {
    let source = "verb buffered_write(ins buffer: Buffer, abs input: Buffer) -> Int { return 0; } verb main() -> Int { erg buffer = Buffer[0]; return buffered_write(buffer: ins buffer, input: abs buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("buffered alias fixture should parse");
    assert!(analyze(&program).is_err());
}

#[test]
fn std_io_read_line_requires_an_explicit_ins_argument() {
    let source = "verb read_line(ins buffer: Buffer) -> Int { return 0; } verb main() -> Int { erg buffer = Buffer[0]; return read_line(buffer: buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("invalid ins call should parse");
    assert!(analyze(&program).is_err());
}

#[test]
fn std_io_cursor_seek_and_copy_contracts_are_semantically_valid() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let resolver = ModuleResolver::new(source_root);
    let exports = exports_module(&resolver, "io").expect("std io facade should resolve");
    assert!(exports.contains("verb", "seek"));
    assert!(exports.contains("verb", "copy"));
    analyze_module(&resolver, "io").expect("cursor utility contracts should be valid");
}

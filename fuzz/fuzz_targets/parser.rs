#![no_main]

use actus::{formatter::format_program, lexer::scan, parser::parse, semantic::analyze};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let source = String::from_utf8_lossy(input);
    let (tokens, _) = scan(&source);
    if let Ok(program) = parse(tokens) {
        let _ = format_program(&program);
        let _ = analyze(&program);
    }
});

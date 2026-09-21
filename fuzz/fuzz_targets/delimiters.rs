#![no_main]

use actus::{lexer::scan, parser::parse};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let source = delimiter_input(input);
    let (tokens, _) = scan(&source);
    let _ = parse(tokens);
});

fn delimiter_input(input: &[u8]) -> String {
    input
        .iter()
        .map(|byte| match byte % 8 {
            0 => '{',
            1 => '}',
            2 => '(',
            3 => ')',
            4 => ';',
            5 => ',',
            6 => ':',
            _ => ' ',
        })
        .collect()
}

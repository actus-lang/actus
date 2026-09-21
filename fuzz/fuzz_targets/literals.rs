#![no_main]

use actus::{lexer::scan, parser::parse};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let source = literal_input(input);
    let (tokens, _) = scan(&source);
    let _ = parse(tokens);
});

fn literal_input(input: &[u8]) -> String {
    let content = String::from_utf8_lossy(input);
    format!("verb main() {{ // {content}\n return \"{content}\"; }}")
}

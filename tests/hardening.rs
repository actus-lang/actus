use std::panic::{AssertUnwindSafe, catch_unwind};

use actus::lexer::scan;
use actus::parser::parse;
use actus::{formatter::format_program, semantic::analyze};

#[test]
fn malformed_input_corpus_never_panics() {
    let corpus = [
        "{",
        "}",
        "verb main( { return 1; }",
        "verb main() { return (1; }",
        "verb main() { return 1; }}",
        "verb main() { return \"unterminated; }",
        "verb main() { /* unterminated",
        "verb main() { erg = ; }",
        "verb main() { loop { break; ",
        "verb main() { consume(a: 1,); }",
    ];

    for source in corpus {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let (tokens, _) = scan(source);
            let _ = parse(tokens);
        }));
        assert!(result.is_ok(), "compiler panicked for malformed input: {source:?}");
    }
}

#[test]
fn generated_inputs_never_panic_across_frontend_stages() {
    let alphabet = b"{}();,:=+-*/\\\" abcdef0123456789";
    let mut state = 0xAC7A5_u64;

    for case in 0..2048 {
        let length = (next_random(&mut state) % 96) as usize;
        let source = (0..length)
            .map(|_| alphabet[(next_random(&mut state) as usize) % alphabet.len()] as char)
            .collect::<String>();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let (tokens, _) = scan(&source);
            if let Ok(program) = parse(tokens) {
                let _ = format_program(&program);
                let _ = analyze(&program);
            }
        }));
        assert!(result.is_ok(), "compiler panicked for generated input {case}: {source:?}");
    }
}

#[test]
fn arbitrary_bytes_never_panic_across_frontend_stages() {
    let mut state = 0xBADC0DE_u64;

    for case in 0..4096 {
        let length = (next_random(&mut state) % 257) as usize;
        let bytes = (0..length).map(|_| next_random(&mut state) as u8).collect::<Vec<_>>();
        let source = String::from_utf8_lossy(&bytes);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let (tokens, _) = scan(&source);
            if let Ok(program) = parse(tokens) {
                let _ = format_program(&program);
                let _ = analyze(&program);
            }
        }));
        assert!(result.is_ok(), "compiler panicked for arbitrary input {case}");
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

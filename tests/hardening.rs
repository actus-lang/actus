use std::panic::{AssertUnwindSafe, catch_unwind};

use actus::lexer::scan;
use actus::parser::parse;

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

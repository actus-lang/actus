use std::hint::black_box;
use std::time::{Duration, Instant};

use actus::lexer::scan;
use actus::parser::parse;

const SAMPLE_SOURCE: &str = include_str!("../tests/fixtures/parser/valid/transfer.act");
const DEFAULT_ITERATIONS: u32 = 10_000;

fn main() {
    let iterations = benchmark_iterations();
    report("lexer", iterations, || {
        let _ = black_box(scan(black_box(SAMPLE_SOURCE)));
    });
    report("parser", iterations, || {
        let (tokens, errors) = scan(SAMPLE_SOURCE);
        assert!(errors.is_empty(), "benchmark fixture must lex cleanly");
        let _ = black_box(parse(black_box(tokens)));
    });
}

fn benchmark_iterations() -> u32 {
    std::env::var("ACTUS_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|iterations| *iterations > 0)
        .unwrap_or(DEFAULT_ITERATIONS)
}

fn report(name: &str, iterations: u32, operation: impl FnMut()) {
    let elapsed = measure(iterations, operation);
    let per_iteration = elapsed / iterations;
    println!("{name}: {iterations} iterations in {elapsed:?} ({per_iteration:?}/iteration)");
}

fn measure(iterations: u32, mut operation: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed()
}

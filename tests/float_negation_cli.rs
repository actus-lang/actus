//! A negative float literal is one constant and unary `-` on a float binding is
//! IEEE negation, on both back ends. Negation used to be lowered with an
//! overflow-checked integer subtraction, which LLVM rejected and Cranelift
//! panicked on.
#![cfg(unix)]

use std::fs;

use actus::cli::run_with_args;

/// Builds `source` with `backend`, runs it, and returns its exit code, or
/// `None` when the build is rejected.
fn build_and_run(name: &str, source: &str, backend: &str) -> Option<i32> {
    let stem = std::env::temp_dir().join(format!("{name}-{backend}-{}", std::process::id()));
    let input = stem.with_extension("act");
    let output = stem.with_extension("bin");
    fs::write(&input, source).expect("write fixture");
    let arguments = [
        "build".to_owned(),
        input.display().to_string(),
        "--backend".to_owned(),
        backend.to_owned(),
        "--emit".to_owned(),
        "exe".to_owned(),
        "-o".to_owned(),
        output.display().to_string(),
    ];
    let code = run_with_args(arguments.into_iter());
    let status = (code == 0).then(|| {
        std::process::Command::new(&output).status().expect("run fixture").code().expect("exit")
    });
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    status
}

fn backends() -> Vec<&'static str> {
    let mut backends = Vec::new();
    if cfg!(feature = "cranelift") {
        backends.push("cranelift");
    }
    if cfg!(feature = "llvm") {
        backends.push("llvm");
    }
    backends
}

fn run_on_every_backend(name: &str, source: &str, expected: i32) {
    for backend in backends() {
        assert_eq!(build_and_run(name, source, backend), Some(expected), "{name} on {backend}");
    }
}

#[test]
fn a_negative_float_literal_is_negative() {
    // Branches 1, 2, 4 and 8 are taken and 16 is not.
    let source = "verb main() -> Int {\n    erg a: f64 = -1.5f64;\n    erg b: f32 = -(2.5f32);\n    erg c: f64 = -1.5;\n    erg result: Int = 0;\n    if a < 0.0f64 { result += 1; }\n    if b < 0.0f32 { result += 2; }\n    if c == a { result += 4; }\n    if -1.5f64 < 0.0f64 { result += 8; }\n    if a > 0.0f64 { result += 16; }\n    return result;\n}\n";
    run_on_every_backend("actus-float-literal", source, 15);
}

#[test]
fn negating_a_float_binding_is_ieee_negation() {
    let source = "verb main() -> Int {\n    erg x: f64 = 1.5f64;\n    erg y: f32 = 2.5f32;\n    erg negated: f64 = -x;\n    erg result: Int = 0;\n    if negated < 0.0f64 { result += 1; }\n    if -negated > 0.0f64 { result += 2; }\n    if -y < 0.0f32 { result += 4; }\n    if -(-x) == x { result += 8; }\n    if -x > 0.0f64 { result += 16; }\n    return result;\n}\n";
    run_on_every_backend("actus-float-binding", source, 15);
}

#[test]
fn a_negative_float_constant_argument_and_return_value_keep_their_sign() {
    let source = "const C: f64 = -2.5f64;\n\nverb flip(erg v: f64) -> f64 {\n    return -v;\n}\n\nverb literal() -> f64 {\n    return -3.5f64;\n}\n\nverb is_negative(erg v: f64) -> Int {\n    if v < 0.0f64 { return 1; }\n    return 0;\n}\n\nverb main() -> Int {\n    erg p: f64 = 2.0f64;\n    erg q: f64 = -1.0f64;\n    erg result: Int = 0;\n    if C < 0.0f64 { result += 1; }\n    if -C > 0.0f64 { result += 2; }\n    if flip(v: p) < 0.0f64 { result += 4; }\n    if literal() < 0.0f64 { result += 8; }\n    result += is_negative(v: q) * 16;\n    return result;\n}\n";
    run_on_every_backend("actus-float-constant", source, 31);
}

#[test]
fn negative_zero_equals_zero() {
    let source = "verb main() -> Int {\n    erg zero: f64 = 0.0f64;\n    erg negative: f64 = -0.0f64;\n    erg result: Int = 0;\n    if negative == zero { result += 1; }\n    if -zero == 0.0f64 { result += 2; }\n    if -0.0f64 == 0.0f64 { result += 4; }\n    if negative < zero { result += 8; }\n    return result;\n}\n";
    run_on_every_backend("actus-float-negative-zero", source, 7);
}

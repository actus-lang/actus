//! Integer literals and arithmetic in widths that are not machine widths
//! (`i33`, `u40`, `i48`, `u63`, ...) keep every bit on every backend.

use super::*;

/// The native backends this test build can emit with.
fn backends() -> Vec<&'static str> {
    [cfg!(feature = "llvm").then_some("llvm"), cfg!(feature = "cranelift").then_some("cranelift")]
        .into_iter()
        .flatten()
        .collect()
}

/// Builds `source` with `backend` in a fresh directory and returns the
/// build output together with that directory.
fn build(name: &str, source: &str, backend: &str) -> (std::process::Output, std::path::PathBuf) {
    let root = std::env::temp_dir()
        .join(format!("actus-odd-width-{name}-{backend}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create test directory");
    fs::write(root.join("main.act"), source).expect("write source");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .current_dir(&root)
        .args(["build", "main.act", "--emit", "exe", "--backend", backend, "-o", "main.out"])
        .output()
        .expect("run actus build");
    (output, root)
}

/// Runs `body` as `main` on every backend and checks its exact output.
fn assert_prints(name: &str, items: &str, body: &str, expected: &[&str]) {
    let source = format!("{items}\nverb main() {{\n{body}\n}}\n");
    let expected = expected.iter().map(|line| format!("{line}\n")).collect::<String>();
    for backend in backends() {
        let (build, root) = build(name, &source, backend);
        assert!(
            build.status.success(),
            "`{name}` must build with {backend}: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let run = Command::new(root.join("main.out")).output().expect("run the built program");
        assert_eq!(run.status.code(), Some(0), "{name} with {backend}");
        assert_eq!(String::from_utf8_lossy(&run.stdout), expected, "{name} with {backend}");
        let _ = fs::remove_dir_all(root);
    }
}

/// Runs `body` as `main` on every backend and checks that it stops with
/// `message` and status 101.
fn assert_traps(name: &str, items: &str, body: &str, message: &str) {
    let source = format!("{items}\nverb main() {{\n{body}\n}}\n");
    for backend in backends() {
        let (build, root) = build(name, &source, backend);
        assert!(build.status.success(), "`{name}` must build with {backend}");
        let run = Command::new(root.join("main.out")).output().expect("run the built program");
        assert_eq!(run.status.code(), Some(101), "{name} with {backend}");
        assert_eq!(
            String::from_utf8_lossy(&run.stderr),
            format!("error: {message}\n"),
            "{name} with {backend}"
        );
        let _ = fs::remove_dir_all(root);
    }
}

/// Checks that `body` is rejected at compile time with `message`.
fn assert_rejected(name: &str, body: &str, message: &str) {
    let source = format!("verb main() {{\n{body}\n}}\n");
    for backend in backends() {
        let (build, root) = build(name, &source, backend);
        assert!(!build.status.success(), "`{name}` must not build with {backend}");
        let stderr = String::from_utf8_lossy(&build.stderr);
        assert!(stderr.contains(message), "{name} with {backend}: {stderr}");
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn signed_odd_width_literals_keep_every_bit_at_their_bounds() {
    assert_prints(
        "signed-bounds",
        "",
        "erg a: i33 = 4294967295i33;\nprint(int: a as i64);\n\
         erg b: i33 = -4294967296i33;\nprint(int: b as i64);\n\
         erg c: i40 = 549755813887i40;\nprint(int: c as i64);\n\
         erg d: i40 = -549755813888i40;\nprint(int: d as i64);\n\
         erg e: i48 = 140737488355327i48;\nprint(int: e as i64);\n\
         erg f: i48 = -140737488355328i48;\nprint(int: f as i64);\n\
         erg g: i63 = 4611686018427387903i63;\nprint(int: g as i64);\n\
         erg h: i63 = -4611686018427387904i63;\nprint(int: h as i64);",
        &[
            "4294967295",
            "-4294967296",
            "549755813887",
            "-549755813888",
            "140737488355327",
            "-140737488355328",
            "4611686018427387903",
            "-4611686018427387904",
        ],
    );
}

#[test]
fn unsigned_odd_width_literals_keep_every_bit_at_their_bounds() {
    assert_prints(
        "unsigned-bounds",
        "",
        "erg a: u33 = 8589934591u33;\nprint(int: a as u64);\n\
         erg b: u40 = 1099511627775u40;\nprint(int: b as u64);\n\
         erg c: u48 = 281474976710655u48;\nprint(int: c as u64);\n\
         erg d: u63 = 9223372036854775807u63;\nprint(int: d as u64);",
        &["8589934591", "1099511627775", "281474976710655", "9223372036854775807"],
    );
}

#[test]
fn machine_width_literals_are_unchanged() {
    assert_prints(
        "control",
        "",
        "erg a: i64 = 9223372036854775807i64;\nprint(int: a);\n\
         erg b: i64 = -9223372036854775808i64;\nprint(int: b);\n\
         erg c: u64 = 18446744073709551615u64;\nprint(int: c);",
        &["9223372036854775807", "-9223372036854775808", "18446744073709551615"],
    );
}

#[test]
fn odd_width_literals_work_as_operands_comparisons_and_print_arguments() {
    assert_prints(
        "operands",
        "",
        "erg k: i40 = 5000000000i40;\n\
         erg sum: i40 = k + 1i40;\nprint(int: sum as i64);\n\
         if sum > 5000000000i40 { print(string: \"greater\"); }\n\
         if sum == 5000000001i40 { print(string: \"equal\"); }\n\
         erg low: i40 = -5000000000i40;\n\
         if low < k { print(string: \"less\"); }\n\
         print(int: low);",
        &["5000000001", "greater", "equal", "less", "-5000000000"],
    );
}

#[test]
fn odd_width_literals_work_in_constants_calls_returns_and_aggregates() {
    assert_prints(
        "positions",
        "const LIMIT: u48 = 281474976710655u48;\n\
         verb widen() -> i40 { return 300000000000i40; }\n\
         verb add(erg left: i40, erg right: i40) -> i40 { return left + right; }\n\
         struct Pair { first: i40, second: u48 }",
        "print(int: LIMIT as u64);\n\
         erg left: i40 = widen();\nerg right: i40 = -100000000000i40;\n\
         erg total: i40 = add(left: left, right: right);\nprint(int: total as i64);\n\
         erg values: Array[i40, 2] = Array[i40, 2]();\n\
         values[0] = 500000000000i40;\nvalues[1] = -500000000000i40;\n\
         print(int: values[0] as i64);\nprint(int: values[1] as i64);\n\
         erg pair: Pair = Pair { first: -500000000000i40, second: 281474976710655u48 };\n\
         print(int: pair.first as i64);\nprint(int: pair.second as u64);",
        &[
            "281474976710655",
            "200000000000",
            "500000000000",
            "-500000000000",
            "-500000000000",
            "281474976710655",
        ],
    );
}

#[test]
fn odd_width_literals_work_in_packed_fields() {
    assert_prints(
        "packed",
        "pack Reg { erg storage: u64; layout little; fields { erg wide: u40 at 0; erg low: i24 at 40; } }",
        "erg reg: Reg = Reg { storage: 0 };\n\
         reg.wide = 1099511627775u40;\nreg.low = -5i24;\n\
         print(int: reg.wide as u64);\nprint(int: reg.low as i64);",
        &["1099511627775", "-5"],
    );
}

#[test]
fn odd_width_arithmetic_stays_inside_its_width() {
    assert_prints(
        "arithmetic",
        "",
        "erg k: i40 = 100000000000i40;\nerg product: i40 = k * 5i40;\nprint(int: product as i64);\n\
         erg top: i40 = 549755813887i40;\nerg wrapped: i40 = top +% 1i40;\nprint(int: wrapped as i64);\n\
         erg clamped: i40 = top +| 5i40;\nprint(int: clamped as i64);\n\
         erg floor: u40 = 0u40;\nerg below: u40 = floor -% 1u40;\nprint(int: below as u64);\n\
         erg held: u40 = floor -| 1u40;\nprint(int: held as u64);\n\
         erg flipped: i40 = ~5i40;\nprint(int: flipped as i64);\n\
         erg minimum: i40 = -549755813888i40;\nerg half: i40 = minimum / 2i40;\nprint(int: half as i64);",
        &[
            "500000000000",
            "-549755813888",
            "549755813887",
            "1099511627775",
            "0",
            "-6",
            "-274877906944",
        ],
    );
}

#[test]
fn odd_width_case_patterns_and_range_bindings_keep_every_bit() {
    assert_prints(
        "control-flow",
        "",
        "erg k: i40 = 5000000000i40;\n\
         erg picked: Int = case k { 7 => 3, 5000000000 => 1, _ => 2, };\nprint(int: picked);\n\
         erg last: i40 = 0i40;\n\
         for erg index: i40 in 500000000000i40..500000000003i40 { last = index; }\n\
         print(int: last as i64);",
        &["1", "500000000002"],
    );
}

#[test]
fn odd_width_addition_overflow_traps_at_the_type_width() {
    assert_traps(
        "add-overflow",
        "",
        "erg top: i40 = 549755813887i40;\nerg next: i40 = top + 1i40;\nprint(int: next as i64);",
        "integer overflow at main.act:4:17",
    );
}

#[test]
fn odd_width_multiplication_overflow_traps_at_the_type_width() {
    assert_traps(
        "mul-overflow",
        "",
        "erg big: i40 = 300000000000i40;\nerg twice: i40 = big * 2i40;\nprint(int: twice as i64);",
        "integer overflow at main.act:4:18",
    );
}

#[test]
fn unsigned_odd_width_subtraction_below_zero_traps() {
    assert_traps(
        "unsigned-underflow",
        "",
        "erg zero: u48 = 0u48;\nerg below: u48 = zero - 1u48;\nprint(int: below as u64);",
        "integer overflow at main.act:4:18",
    );
}

#[test]
fn odd_width_signed_division_overflow_traps_at_the_type_width() {
    // The division runs inside a verb so that no backend folds the operands.
    assert_traps(
        "division-overflow",
        "verb divide(abs left: i40, abs right: i40) -> i40 { return left / right; }",
        "erg minimum: i40 = -549755813888i40;\nerg divisor: i40 = -1i40;\n\
         print(int: divide(left: abs minimum, right: abs divisor) as i64);",
        "integer overflow at main.act:1:60",
    );
}

#[test]
fn narrowing_cast_out_of_the_odd_width_range_traps() {
    assert_traps(
        "cast-overflow",
        "",
        "erg source: i64 = 549755813888i64;\nerg narrow: i40 = source as i40;\nprint(int: narrow as i64);",
        "integer value out of range at main.act:4:19",
    );
}

#[test]
fn odd_width_literals_just_outside_their_range_are_rejected() {
    for (name, body, message) in [
        (
            "above-i40",
            "erg k: i40 = 549755813888i40;",
            "literal `549755813888` is outside the range of `i40`",
        ),
        (
            "below-i40",
            "erg k: i40 = -549755813889i40;",
            "literal `-549755813889` is outside the range of `i40`",
        ),
        (
            "above-u40",
            "erg k: u40 = 1099511627776u40;",
            "literal `1099511627776` is outside the range of `u40`",
        ),
        (
            "above-u63",
            "erg k: u63 = 9223372036854775808u63;",
            "literal `9223372036854775808` is outside the range of `u63`",
        ),
        (
            "above-i33",
            "erg k: i33 = 4294967296i33;",
            "literal `4294967296` is outside the range of `i33`",
        ),
    ] {
        assert_rejected(name, body, message);
    }
}

#[test]
fn suffixed_hex_literals_print_their_values_on_every_backend() {
    assert_prints(
        "hex-suffix",
        "",
        "erg a: u8 = 0xFFu8;\nprint(int: a as i64);\n\
         erg b: i8 = 0x7Fi8;\nprint(int: b as i64);\n\
         erg c: u32 = 0xFFFFFFFFu32;\nprint(int: c as i64);\n\
         erg d: i40 = 0xFFi40;\nprint(int: d as i64);\n\
         erg e: u8 = 0XFFu8;\nprint(int: e as i64);",
        &["255", "127", "4294967295", "255", "255"],
    );
}

#[test]
fn suffixed_hex_literals_are_range_checked_against_their_suffix() {
    assert_rejected(
        "hex-u8-range",
        "erg a = 0x100u8;",
        "literal `0x100` is outside the range of `u8`",
    );
    assert_rejected(
        "hex-i8-range",
        "erg a = 0xFFi8;",
        "literal `0xFF` is outside the range of `i8`",
    );
}

#[test]
fn hex_literal_without_digits_is_rejected_even_with_a_suffix() {
    assert_rejected("hex-no-digits", "erg a = 0xu8;", "invalid hexadecimal literal `0xu8`");
}

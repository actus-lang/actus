#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
#[test]
fn executes_contiguous_array_reads_and_writes_natively() {
    let status = run_array_fixture(
        "array-read-write",
        "verb main() -> Int { erg values: Array[Int, 4] = Array[Int, 4](); values[1] = 41; return values[1] + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn reads_first_and_last_buffer_bytes_with_dynamic_indexing() {
    let status = run_array_fixture(
        "buffer-index",
        "verb main() -> Int { erg bytes: Buffer = Buffer[0]; append(bytes, 20); append(bytes, 22); erg first = bytes[0]; erg last = bytes[1]; return first + last; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn lowers_adr44_integer_comparisons_remainder_bitwise_and_shifts() {
    let status = run_array_fixture(
        "adr44-integer-operators",
        "verb main() -> Int { erg left: u8 = 13; erg right: u8 = 5; erg count: u8 = 3; erg remainder = left % right; erg bits = (left & right) ^ left; erg shifted = right << count; return case left == left { true => (remainder as Int) + (bits as Int) + (shifted as Int) - 9, _ => 0, }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn lowers_bitwise_not_and_signed_right_shift() {
    let status = run_array_fixture(
        "adr44-unary-and-signed-shift",
        "verb main() -> Int { erg value: i8 = -8; erg count: u8 = 2; erg shifted = value >> count; erg mask: u8 = 0; erg inverted = ~mask; erg expected: u8 = 255; return case inverted == expected { true => shifted as Int + 44, _ => 0, }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn lowers_float_equality_and_inequality() {
    let status = run_array_fixture(
        "adr44-float-equality",
        "verb main() -> Int { erg left: f32 = 1.5; erg same: f32 = 1.5; erg other: f32 = 2.5; return case (left == same) && (left != other) { true => 42, _ => 0, }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn traps_on_integer_remainder_by_zero() {
    let status = run_array_fixture(
        "adr44-remainder-zero",
        "verb main() -> Int { erg value = 42; erg divisor = 0; return value % divisor; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "remainder by zero should trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn traps_on_shift_count_out_of_range() {
    let status = run_array_fixture(
        "adr44-shift-range",
        "verb main() -> Int { erg value: u8 = 1; erg count: u8 = 8; return value << count; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "shift count overflow should trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn short_circuit_and_skips_an_unreachable_rhs() {
    let status = run_array_fixture(
        "adr44-short-circuit-and",
        "verb fail() -> Bool { erg zero = 0; return 1 / zero == 0; } verb main() -> Int { return case (1 > 2) && fail() { false => 42, _ => 0, }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn short_circuit_or_skips_an_unreachable_rhs() {
    let status = run_array_fixture(
        "adr44-short-circuit-or",
        "verb fail() -> Bool { erg zero = 0; return 1 / zero == 0; } verb main() -> Int { return case (1 < 2) || fail() { true => 42, _ => 0, }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn lowers_logical_not_as_a_bool_value() {
    let status = run_array_fixture(
        "adr44-logical-not",
        "verb main() -> Int { return case !(1 > 2) { true => 42, _ => 0, }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn restores_an_ins_buffer_when_short_circuit_skips_the_rhs() {
    let status = run_array_fixture(
        "adr44-short-circuit-ins-skip",
        "verb touch(ins bytes: Buffer) -> Bool { append(bytes, 7); return 1 < 2; } verb main() -> Int { erg bytes: Buffer = Buffer[0]; erg skipped = (1 > 2) && touch(bytes: ins bytes); append(bytes, 42); return bytes[0]; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn restores_an_ins_buffer_when_short_circuit_evaluates_the_rhs() {
    let status = run_array_fixture(
        "adr44-short-circuit-ins-run",
        "verb touch(ins bytes: Buffer) -> Bool { append(bytes, 7); return 1 < 2; } verb main() -> Int { erg bytes: Buffer = Buffer[0]; erg evaluated = (1 < 2) && touch(bytes: ins bytes); return bytes[0]; }",
    );
    assert_eq!(status.code(), Some(7));
}

#[cfg(unix)]
#[test]
fn preserves_nested_case_loop_control_after_short_circuit_evaluation() {
    let status = run_array_fixture(
        "adr44-short-circuit-case-loop",
        "verb main() -> Int { erg flag = 1; loop { case flag { 1 => { case (1 < 2) && (2 < 3) { true => { break; }, _ => { continue; }, }; }, _ => { continue; }, }; } return 42; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn preserves_drop_cleanup_inside_an_evaluated_short_circuit_rhs() {
    let status = run_array_fixture(
        "adr44-short-circuit-cleanup",
        "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } verb make() -> Bool { erg item = Counter { value: 7, }; return 1 < 2; } verb main() -> Int { erg evaluated = (1 < 2) && make(); return 42; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn accepts_u8_and_u32_dynamic_indices() {
    let status = run_array_fixture(
        "unsigned-indices",
        "verb main() -> Int { erg values: Array[u8, 4] = Array[u8, 4](); values[1] = 20; erg array_index: u32 = 1; erg bytes: Buffer = Buffer[0]; append(bytes, 22); erg buffer_index: u8 = 0; return values[array_index] + bytes[buffer_index]; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_checked_integer_casts() {
    let status = run_array_fixture(
        "checked-casts",
        "verb main() -> Int { erg value = 41; erg narrowed = value as u8; return narrowed as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn traps_on_runtime_integer_cast_overflow() {
    let status = run_array_fixture(
        "checked-cast-overflow",
        "verb main() -> Int { erg value = 256; erg narrowed = value as u8; return narrowed as Int; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "integer overflow should be a native trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn traps_on_runtime_integer_cast_underflow() {
    let status = run_array_fixture(
        "checked-cast-underflow",
        "verb main() -> Int { erg value = -1; erg narrowed = value as u8; return narrowed as Int; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "integer underflow should be a native trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn traps_deterministically_on_buffer_bounds_failure() {
    let status = run_array_fixture(
        "buffer-index-bounds",
        "verb main() -> Int { erg bytes: Buffer = Buffer[0]; append(bytes, 42); return bytes[1]; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "buffer bounds failure should be a native trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn passes_an_array_as_an_exclusive_in_place_parameter() {
    let status = run_array_fixture(
        "array-ins-parameter",
        "verb mutate(ins values: Array[Int, 2]) { values[1] = 41; } verb main() -> Int { erg values: Array[Int, 2] = Array[Int, 2](); mutate(values: ins values); return values[1] + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn checks_dynamic_values_before_writing_u8_pack_fields() {
    let status = run_array_fixture(
        "pack-u8-range",
        "pack Byte { erg storage: u8; layout little; fields { erg payload: u8 at 0; } } verb main() -> Int { erg byte = Byte { storage: 0, }; erg input = 41; byte.payload = input; return byte.payload; }",
    );
    assert_eq!(status.code(), Some(41));
}

#[cfg(unix)]
#[test]
fn traps_when_a_dynamic_value_exceeds_u8_range() {
    let status = run_array_fixture(
        "pack-u8-overflow",
        "pack Byte { erg storage: u8; layout little; fields { erg payload: u8 at 0; } } verb main() -> Int { erg byte = Byte { storage: 0, }; erg input = 256; byte.payload = input; return byte.payload; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "u8 overflow should be a native trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn traps_deterministically_on_dynamic_array_bounds_failure() {
    let status = run_array_fixture(
        "array-bounds",
        "verb main() -> Int { erg values: Array[Int, 4] = Array[Int, 4](); erg index = 4; return values[index]; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "bounds failure should be a native trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn copies_aggregate_array_elements_without_copying_the_array() {
    let status = run_array_fixture(
        "array-aggregate",
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg points: Array[Point, 2] = Array[Point, 2](); points[1] = Point { x: 19, y: 23, }; return points[1].x + points[1].y; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn places_contiguous_arrays_in_an_arena_without_reallocation() {
    let status = run_array_fixture(
        "array-arena",
        "verb main() -> Int { erg arena: Arena[128] = Arena[128](); erg values: Array[Int, 4] = arena.place(value: Array[Int, 4]()); values[2] = 41; return values[2] + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn mutates_indexed_pack_fields_in_place() {
    let status = run_array_fixture(
        "array-pack",
        "pack Control { erg storage: u8; layout little; fields { erg enabled: u1 at 0; abs _reserved: u7 at 1; } } verb main() -> Int { erg controls: Array[Control, 2] = Array[Control, 2](); controls[1] = Control { storage: 0, }; controls[1].enabled = 1; return controls[1].enabled; }",
    );
    assert_eq!(status.code(), Some(1));
}

#[cfg(unix)]
#[test]
fn mutates_an_indexed_ins_slot_and_reuses_the_array_owner() {
    let status = run_array_fixture(
        "array-ins-slot",
        "verb mutate(ins slot: Int) { slot = 42; } verb main() -> Int { erg values: Array[Int, 2] = Array[Int, 2](); mutate(slot: ins values[1]); return values[1]; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn emits_identical_objects_for_repeated_array_builds() {
    let root = std::env::temp_dir().join(format!("actus-array-repeat-{}", std::process::id()));
    let input = root.with_extension("act");
    let first = root.with_extension("first.o");
    let second = root.with_extension("second.o");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[Int, 2] = Array[Int, 2](); values[1] = 41; return values[1] + 1; }",
    )
    .expect("write deterministic array fixture");
    for output in [&first, &second] {
        let result = run_with_args(
            vec![
                "build".to_owned(),
                input.display().to_string(),
                "--emit".to_owned(),
                "obj".to_owned(),
                "-o".to_owned(),
                output.display().to_string(),
            ]
            .into_iter(),
        );
        assert_eq!(result, 0);
    }
    assert_eq!(
        fs::read(&first).expect("read first object"),
        fs::read(&second).expect("read second object")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[cfg(unix)]
fn run_array_fixture(name: &str, source: &str) -> std::process::ExitStatus {
    let root = std::env::temp_dir().join(format!("actus-{name}-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, source).expect("write array fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run array fixture");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    status
}

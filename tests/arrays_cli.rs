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
fn executes_typed_integer_literals_natively() {
    let status =
        run_array_fixture("typed-integer-literals", "verb main() -> u32 { return 1u32 + 2u32; }");
    assert_eq!(status.code(), Some(3));
}

#[cfg(unix)]
#[test]
fn executes_boolean_literals_and_short_circuit_logic_natively() {
    let status = run_array_fixture(
        "boolean-literals",
        "verb main() -> Int { erg linked: Bool = false; erg result: Int = if true && !linked { 42 } else { 0 }; return result; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_const_generic_array_layouts_natively() {
    let status = run_array_fixture(
        "const-generic-array-layout",
        "struct Cell { erg charge: u8, } struct Fabric[N: Usize] { erg cells: Array[Cell, N], } verb main() -> Int { erg fabric: Fabric[2] = Fabric[2] { cells: Array[Cell, 2](), }; fabric.cells[0].charge = 41u8; return fabric.cells[0].charge as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn inlines_typed_named_constants_natively() {
    let status = run_array_fixture(
        "typed-named-constant",
        "const ANSWER: u32 = 41u32; verb main() -> Int { return ANSWER as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn inlines_chained_named_constants_natively() {
    let status = run_array_fixture(
        "chained-typed-named-constants",
        "const OFFSET: Int = 41; const ANSWER: Int = OFFSET + 1; verb main() -> Int { return ANSWER; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_value_producing_if_branches_natively() {
    let status = run_array_fixture(
        "conditional-expression",
        "verb main() -> Int { erg selected = if 1u32 < 2u32 { 41 } else { 1 }; return selected + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn does_not_evaluate_the_non_selected_if_branch() {
    let status = run_array_fixture(
        "conditional-expression-short-branch",
        "verb main() -> Int { return if 1 < 2 { 42 } else { 1 / 0 }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_nested_value_producing_conditionals() {
    let status = run_array_fixture(
        "nested-conditional-expression",
        "verb main() -> Int { return if 1 < 2 { if 2 < 3 { 42 } else { 0 } } else { 0 }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_compound_assignments_natively() {
    let status = run_array_fixture(
        "compound-assignments",
        "verb main() -> Int { erg value: u8 = 5u8; value += 7u8; value -= 2u8; value *= 3u8; value /= 5u8; value %= 4u8; value &= 3u8; value |= 4u8; value ^= 1u8; value <<= 1u8; value >>= 1u8; return value as Int; }",
    );
    assert_eq!(status.code(), Some(7));
}

#[cfg(unix)]
#[test]
fn executes_type_directed_unsuffixed_integer_operations_natively() {
    let status = run_array_fixture(
        "type-directed-integer-operations",
        "verb main() -> Int { erg index: u32 = 0; index += 41; return (index + 1) as Int; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_index_compound_assignment_with_one_address_calculation() {
    let status = run_array_fixture(
        "compound-index-assignment",
        "verb main() -> Int { erg values: Array[u8, 2] = Array[u8, 2](); values[0u8] = 5u8; values[0u8] += 37u8; return values[0u8] as Int; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_buffer_index_compound_assignment() {
    let status = run_array_fixture(
        "compound-buffer-assignment",
        "verb main() -> Int { erg bytes: Buffer = Buffer[0]; append(bytes, 5u8); bytes[0u8] += 37u8; return bytes[0u8] as Int; }",
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
fn lowers_typed_float_literals_at_the_declared_width() {
    let status = run_array_fixture(
        "typed-float-literals",
        "verb main() -> Int { erg value: f32 = 1.5f32; return case value == 1.5f32 { true => 42, _ => 0, }; }",
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
fn executes_unsuffixed_integer_comparison_and_indexing_natively() {
    let status = run_array_fixture(
        "unsuffixed-comparison-and-index",
        "verb main() -> Int { erg values: Array[u8, 2] = Array[u8, 2](); values[0] = 41; erg index: u32 = 0; return case index < 1 { true => (values[index] as Int) + 1, _ => 0, }; }",
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
fn executes_nested_array_struct_place_assignment() {
    let status = run_array_fixture(
        "nested-array-struct-place",
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg points: Array[Point, 2] = Array[Point, 2](); points[1] = Point { x: 19, y: 3, }; points[1].x = 40; points[1].x += 2; return points[1].x + points[1].y; }",
    );
    assert_eq!(status.code(), Some(45));
}

#[cfg(unix)]
#[test]
fn preserves_ins_loan_for_a_nested_array_struct_place() {
    let status = run_array_fixture(
        "nested-array-struct-ins",
        "struct Point { x: Int, } verb mutate(ins slot: Int) { slot = 42; } verb main() -> Int { erg points: Array[Point, 2] = Array[Point, 2](); points[1] = Point { x: 0, }; mutate(slot: ins points[1].x); return points[1].x; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_nested_place_in_a_generic_array_aggregate() {
    let status = run_array_fixture(
        "nested-generic-array-place",
        "struct Box[T] { item: T, } verb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1] = Box[Int] { item: 7, }; boxes[1].item = 35; return boxes[1].item + 7; }",
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

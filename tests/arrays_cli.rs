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
fn executes_indexed_pack_field_reads_and_writes_natively() {
    let status = run_array_fixture(
        "indexed-pack-field-read-write",
        "pack Example { erg storage: Array[u8, 8]; layout little; fields { erg links: Array[u32, 2] at 0; } } verb main() -> Int { erg example = Example { storage: Array[u8, 8](), }; erg index: u32 = 1u32; example.links[index] = 41u32; return example.links[index] as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_compound_indexed_pack_field_updates_natively() {
    let status = run_array_fixture(
        "compound-indexed-pack-field-update",
        "pack Example { erg storage: Array[u8, 8]; layout little; fields { erg links: Array[u32, 2] at 0; } } verb main() -> Int { erg example = Example { storage: Array[u8, 8](), }; erg index: u32 = 1u32; example.links[index] = 40u32; example.links[index] += 1u32; return example.links[index] as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_crc32_intrinsic_through_the_native_runtime() {
    let status = run_array_fixture(
        "crc32-intrinsic",
        "verb main() -> Int { erg buffer: Buffer = Buffer[4]; erg checksum: Int = crc32(buffer: abs buffer, start: 0, end: 0); drop(buffer); return if checksum == 0 { 42 } else { 0 }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_crc32_match_intrinsic_through_the_native_runtime() {
    let status = run_array_fixture(
        "crc32-match-intrinsic",
        "verb main() -> Int { erg buffer: Buffer = Buffer[4]; erg matched: Int = crc32_matches(buffer: abs buffer, start: 0, end: 0, expected: 0); drop(buffer); return if matched == 1 { 42 } else { 0 }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_fixed_frame_validation_intrinsic_through_the_native_runtime() {
    let status = run_array_fixture(
        "fixed-frame-validation-intrinsic",
        "verb main() -> Int { erg buffer: Buffer = Buffer[9]; erg valid: Int = validate_fixed_frame(buffer: abs buffer, little: 1, version_offset: 0, expected_version: 1, payload_offset: 2, payload_length: 3, checksum_start: 0, checksum_end: 5, checksum_offset: 5); drop(buffer); return valid; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn executes_generated_serialization_validator_natively() {
    let status = run_array_fixture(
        "generated-serialization-validator",
        "pack FramePack { erg storage: Array[u8, 32]; layout little; fields { erg word_0: u64 at 0; erg word_1: u64 at 64; erg word_2: u64 at 128; erg word_3: u64 at 192; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 16; checksum crc32 over 0 .. 18 at 18; } verb main() -> Int { erg buffer: Buffer = Buffer[32]; erg version: u16 = 1u16; erg valid: Int = frame_validate(frame: abs buffer, expected_version: abs version); drop(buffer); return valid; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn executes_generated_serialization_encoder_natively() {
    let status = run_array_fixture(
        "generated-serialization-encoder",
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg word_0: u8 at 0; erg word_1: u8 at 8; erg word_2: u8 at 16; erg word_3: u8 at 24; erg word_4: u8 at 32; erg word_5: u8 at 40; erg word_6: u8 at 48; erg word_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg source = FramePack { storage: Array[u8, 8](), }; source.storage[0] = 41u8; source.storage[7] = 43u8; erg output = Buffer[0]; erg result = frame_encode(value: abs source, output: ins output); return case dat result { Result.Ok(_) => output[0] as Int + output[7] as Int, Result.Err(_) => 99, }; }",
    );
    assert_eq!(status.code(), Some(84));
}

#[cfg(unix)]
#[test]
fn executes_generated_serialization_decoder_natively() {
    let status = run_array_fixture(
        "generated-serialization-decoder",
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg word_0: u8 at 0; erg word_1: u8 at 8; erg word_2: u8 at 16; erg word_3: u8 at 24; erg word_4: u8 at 32; erg word_5: u8 at 40; erg word_6: u8 at 48; erg word_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg input = Buffer[0]; append(input, 1u8); append(input, 0u8); append(input, 7u8); append(input, 134u8); append(input, 38u8); append(input, 231u8); append(input, 96u8); append(input, 43u8); erg version: u16 = 1u16; erg result = frame_decode(input: abs input, expected_version: abs version); return case dat result { Result.Ok(value) => value.storage[0] as Int + value.storage[7] as Int, Result.Err(_) => 99, }; }",
    );
    assert_eq!(status.code(), Some(44));
}

#[cfg(unix)]
#[test]
fn rejects_short_input_in_generated_serialization_decoder() {
    let status = run_array_fixture(
        "generated-serialization-decoder-short-input",
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg word_0: u8 at 0; erg word_1: u8 at 8; erg word_2: u8 at 16; erg word_3: u8 at 24; erg word_4: u8 at 32; erg word_5: u8 at 40; erg word_6: u8 at 48; erg word_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg input = Buffer[0]; erg version: u16 = 1u16; erg result = frame_decode(input: abs input, expected_version: abs version); return case dat result { Result.Ok(_) => 1, Result.Err(_) => 0, }; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn rejects_invalid_version_in_generated_serialization_decoder() {
    let status = run_array_fixture(
        "generated-serialization-decoder-invalid-version",
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg word_0: u8 at 0; erg word_1: u8 at 8; erg word_2: u8 at 16; erg word_3: u8 at 24; erg word_4: u8 at 32; erg word_5: u8 at 40; erg word_6: u8 at 48; erg word_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg input = Buffer[0]; append(input, 1u8); append(input, 0u8); append(input, 7u8); append(input, 134u8); append(input, 38u8); append(input, 231u8); append(input, 96u8); append(input, 43u8); erg version: u16 = 2u16; erg result = frame_decode(input: abs input, expected_version: abs version); return case dat result { Result.Ok(_) => 1, Result.Err(_) => 0, }; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn rejects_invalid_checksum_in_generated_serialization_decoder() {
    let status = run_array_fixture(
        "generated-serialization-decoder-invalid-checksum",
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg word_0: u8 at 0; erg word_1: u8 at 8; erg word_2: u8 at 16; erg word_3: u8 at 24; erg word_4: u8 at 32; erg word_5: u8 at 40; erg word_6: u8 at 48; erg word_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg input = Buffer[0]; append(input, 1u8); append(input, 0u8); append(input, 7u8); append(input, 134u8); append(input, 38u8); append(input, 231u8); append(input, 97u8); append(input, 43u8); erg version: u16 = 1u16; erg result = frame_decode(input: abs input, expected_version: abs version); return case dat result { Result.Ok(_) => 1, Result.Err(_) => 0, }; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn executes_generated_serialization_migration_natively() {
    let status = run_array_fixture(
        "generated-serialization-migration",
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg word_0: u8 at 0; erg word_1: u8 at 8; erg word_2: u8 at 16; erg word_3: u8 at 24; erg word_4: u8 at 32; erg word_5: u8 at 40; erg word_6: u8 at 48; erg word_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg input = Buffer[0]; append(input, 1u8); append(input, 0u8); append(input, 7u8); append(input, 134u8); append(input, 38u8); append(input, 231u8); append(input, 96u8); append(input, 43u8); erg output = Buffer[0]; erg from_version: u16 = 1u16; erg to_version: u16 = 2u16; erg result = frame_migrate(input: abs input, from_version: abs from_version, to_version: abs to_version, output: ins output); return case dat result { Result.Ok(_) => output[0] as Int + output[7] as Int, Result.Err(_) => 99, }; }",
    );
    assert_eq!(status.code(), Some(45));
}

#[cfg(unix)]
#[test]
fn matches_reference_bytes_after_generated_serialization_migration() {
    let status = run_array_fixture(
        "generated-serialization-migration-reference-bytes",
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg word_0: u8 at 0; erg word_1: u8 at 8; erg word_2: u8 at 16; erg word_3: u8 at 24; erg word_4: u8 at 32; erg word_5: u8 at 40; erg word_6: u8 at 48; erg word_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg input = Buffer[0]; append(input, 1u8); append(input, 0u8); append(input, 7u8); append(input, 134u8); append(input, 38u8); append(input, 231u8); append(input, 96u8); append(input, 43u8); erg output = Buffer[0]; erg from_version: u16 = 1u16; erg to_version: u16 = 2u16; erg result = frame_migrate(input: abs input, from_version: abs from_version, to_version: abs to_version, output: ins output); return case dat result { Result.Ok(count) => if count == 8u32 && output[0] == 2u8 && output[1] == 0u8 && output[2] == 7u8 && output[3] == 223u8 && output[4] == 152u8 && output[5] == 161u8 && output[6] == 98u8 && output[7] == 43u8 { 0 } else { 1 }, Result.Err(_) => 2, }; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn emits_object_and_executable_for_the_same_serialization_contract() {
    let root =
        std::env::temp_dir().join(format!("actus-serialization-parity-{}", std::process::id()));
    let input = root.with_extension("act");
    let object = root.with_extension("o");
    let executable = root.with_extension("bin");
    let source = "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { erg byte_0: u8 at 0; erg byte_1: u8 at 8; erg byte_2: u8 at 16; erg byte_3: u8 at 24; erg byte_4: u8 at 32; erg byte_5: u8 at 40; erg byte_6: u8 at 48; erg byte_7: u8 at 56; } } serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; checksum crc32 over 0 .. 3 at 3; } verb main() -> Int { erg source = FramePack { storage: Array[u8, 8](), }; erg output = Buffer[0]; erg result = frame_encode(value: abs source, output: ins output); return case dat result { Result.Ok(count) => if count == 8u32 { 0 } else { 1 }, Result.Err(_) => 2, }; }";
    fs::write(&input, source).expect("write serialization parity fixture");

    let object_result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "obj".to_owned(),
            "-o".to_owned(),
            object.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(object_result, 0);
    assert!(fs::metadata(&object).expect("object should be emitted").len() > 0);

    let executable_result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            executable.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(executable_result, 0);
    let status = std::process::Command::new(&executable)
        .status()
        .expect("serialization parity executable should run");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(object);
    let _ = fs::remove_file(executable);
}

#[cfg(unix)]
#[test]
fn executes_inferred_abs_and_ins_roles_natively() {
    let status = run_array_fixture(
        "inferred-ownership-roles",
        "verb read(abs value: Int) -> Int { return 41; } verb update(ins value: Int) -> Int { value += 1; return 1; } verb main() -> Int { erg source = 1; abs view = ref source; erg first = read(value: view); ins mutable = 0; erg updated = update(value: mutable); return first + updated; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_generated_scalar_abs_argument_natively() {
    let status = run_array_fixture(
        "generated-scalar-abs-argument",
        "verb read(abs value: u32) -> Int { return value as Int; } verb main() -> Int { erg index: u32 = 41u32; return read(value: abs (index + 1u32)); }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_generated_indexed_scalar_abs_argument_natively() {
    let status = run_array_fixture(
        "generated-indexed-scalar-abs-argument",
        "verb read(abs value: u32) -> Int { return value as Int; } verb main() -> Int { erg values: Array[u32, 2] = Array[u32, 2](); values[1] = 41u32; erg index: u32 = 1u32; return read(value: abs values[index]) + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_generated_named_offset_scalar_abs_argument_natively() {
    let status = run_array_fixture(
        "generated-named-offset-scalar-abs-argument",
        "const OFFSET: u32 = 1u32; verb read(abs value: u32) -> Int { return value as Int; } verb main() -> Int { erg values: Array[u32, 2] = Array[u32, 2](); values[OFFSET] = 41u32; return read(value: abs values[OFFSET]) + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn emits_object_and_executable_for_generated_scalar_abs_argument() {
    let root = std::env::temp_dir()
        .join(format!("actus-generated-scalar-abs-parity-{}", std::process::id()));
    let input = root.with_extension("act");
    let object = root.with_extension("o");
    let executable = root.with_extension("bin");
    let source = "verb read(abs value: u32) -> Int { return value as Int; } verb main() -> Int { erg index: u32 = 41u32; return read(value: abs (index + 1u32)); }";
    fs::write(&input, source).expect("write generated scalar parity fixture");

    let object_result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "obj".to_owned(),
            "-o".to_owned(),
            object.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(object_result, 0);
    assert!(fs::metadata(&object).expect("object should be emitted").len() > 0);

    let executable_result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            executable.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(executable_result, 0);
    let status = std::process::Command::new(&executable)
        .status()
        .expect("generated scalar parity executable should run");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(object);
    let _ = fs::remove_file(executable);
}

#[cfg(unix)]
#[test]
fn executes_indexed_array_backed_pack_storage_natively() {
    let status = run_array_fixture(
        "array-backed-pack-storage",
        "pack CacheLine { erg storage: Array[u8, 4]; layout little; fields { erg byte_0: u8 at 0; erg byte_1: u8 at 8; erg byte_2: u8 at 16; erg byte_3: u8 at 24; } } verb main() -> Int { erg line = CacheLine { storage: Array[u8, 4](), }; line.storage[1] = 40u8; line.byte_1 = 41u8; line.byte_1 += 0u8; return line.byte_1 as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_array_backed_pack_bit_field_access_natively() {
    let status = run_array_fixture(
        "array-backed-pack-bit-field",
        "pack Flags { erg storage: Array[u8, 1]; layout little; fields { erg enabled: u1 at 0; abs _reserved: u7 at 1 = 0; } } verb main() -> Int { erg flags = Flags { storage: Array[u8, 1](), }; flags.enabled = 1; return flags.enabled as Int + 41; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn returns_array_backed_pack_through_native_return_slot() {
    let status = run_array_fixture(
        "array-backed-pack-return",
        "pack Frame { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } verb make() -> Frame { erg frame = Frame { storage: Array[u8, 2](), }; frame.marker = 41u8; return frame; } verb main() -> Int { erg frame = make(); return frame.marker as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn respects_big_endian_array_backed_pack_fields() {
    let status = run_array_fixture(
        "array-backed-pack-big-endian",
        "pack Word { erg storage: Array[u8, 2]; layout big; fields { erg high: u8 at 8; erg low: u8 at 0; } } verb main() -> Int { erg word = Word { storage: Array[u8, 2](), }; word.high = 41u8; return word.high as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn lowers_unaligned_multi_byte_pack_fields_natively() {
    let status = run_array_fixture(
        "array-backed-pack-unaligned-field",
        "pack Slice { erg storage: Array[u8, 2]; layout little; fields { abs _prefix: u2 at 0 = 0; erg payload: u12 at 2; abs _suffix: u2 at 14 = 0; } } verb main() -> Int { erg slice = Slice { storage: Array[u8, 2](), }; slice.payload = 2748u16; return if slice.payload == 2748u16 { 42 } else { 0 }; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn snapshots_array_backed_pack_bytes_through_buffer() {
    let status = run_array_fixture(
        "array-backed-pack-snapshot",
        "pack Frame { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } verb snapshot(ins output: Buffer, abs frame: Frame) { append(output, frame.storage[0]); append(output, frame.storage[1]); } verb main() -> Int { erg frame = Frame { storage: Array[u8, 2](), }; frame.marker = 41u8; erg output = Buffer[0]; snapshot(output: ins output, frame: abs frame); return output[0] as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn restores_array_backed_pack_bytes_from_buffer() {
    let status = run_array_fixture(
        "array-backed-pack-restore",
        "pack Frame { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } verb snapshot(ins output: Buffer, abs frame: Frame) { append(output, frame.storage[0]); append(output, frame.storage[1]); } verb restore(ins frame: Frame, abs input: Buffer) { frame.storage[0] = input[0]; frame.storage[1] = input[1]; } verb main() -> Int { erg source = Frame { storage: Array[u8, 2](), }; source.marker = 41u8; source.tail = 1u8; erg output = Buffer[0]; snapshot(output: ins output, frame: abs source); erg target = Frame { storage: Array[u8, 2](), }; restore(frame: ins target, input: abs output); return target.marker as Int + target.tail as Int; }",
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
fn executes_bounded_for_range_natively() {
    let status = run_array_fixture(
        "bounded-for-range",
        "verb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 0u32 .. 8u32 { total += index; } return total as Int; }",
    );
    assert_eq!(status.code(), Some(28));
}

#[cfg(unix)]
#[test]
fn executes_bounded_repeat_as_the_for_equivalent() {
    let status = run_array_fixture(
        "bounded-repeat",
        "verb main() -> Int { erg total: u32 = 0u32; repeat erg index: u32 in 0u32 .. 8u32 { total += index; } return total as Int - 28; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn preserves_for_range_continue_break_and_reversed_bounds() {
    let status = run_array_fixture(
        "bounded-for-control-flow",
        "verb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 4u32 .. 1u32 { total += 100u32; } for erg index: u32 in 0u32 .. 8u32 { if index == 2u32 { continue; } if index == 5u32 { break; } total += 1u32; } return total as Int; }",
    );
    assert_eq!(status.code(), Some(4));
}

#[cfg(unix)]
#[test]
fn executes_fixed_array_index_iteration_natively() {
    let status = run_array_fixture(
        "fixed-array-for-index",
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[0] = 10u32; values[1] = 20u32; values[2] = 12u32; erg total: u32 = 0u32; for erg index: u32 in values { total += values[index]; } return total as Int; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_pack_backed_array_iteration_natively() {
    let status = run_array_fixture(
        "pack-array-for-index",
        "pack Frame { erg storage: Array[u8, 4]; layout little; fields { abs byte_0: u8 at 0; abs byte_1: u8 at 8; abs byte_2: u8 at 16; abs byte_3: u8 at 24; } } verb main() -> Int { erg frame = Frame { storage: Array[u8, 4](), }; frame.storage[0] = 20u8; frame.storage[1] = 22u8; erg total: u8 = 0u8; for erg index: u32 in frame.storage { total += frame.storage[index]; } return total as Int; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_explicit_scalar_copy_natively() {
    let status = run_array_fixture(
        "explicit-scalar-copy",
        "verb main() -> Int { erg value: u32 = 41u32; erg repeated: u32 = copy(value: abs value); return repeated as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_direct_and_computed_fixed_slot_selection_natively() {
    let status = run_array_fixture(
        "direct-and-computed-fixed-slot-selection",
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2] = 41u32; erg index: u32 = 2u32; erg direct: u32 = values[2]; erg computed: u32 = values[index]; return (direct + computed) as Int + 1; }",
    );
    assert_eq!(status.code(), Some(83));
}

#[cfg(unix)]
#[test]
fn lowers_maximum_u64_literal_natively() {
    let status = run_array_fixture(
        "maximum-u64-literal",
        "verb main() -> Int { erg maximum: u64 = 18446744073709551615u64; if maximum == 18446744073709551615u64 { return 0; } return 1; }",
    );
    assert_eq!(status.code(), Some(0));
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
fn returns_from_statement_if_with_the_enclosing_native_type() {
    let status = run_array_fixture(
        "statement-if-return",
        "verb main() -> Int { if true { return 41; } return 0; }",
    );
    assert_eq!(status.code(), Some(41));
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
fn executes_const_generic_array_argument_in_a_generic_verb() {
    let status = run_array_fixture(
        "const-generic-array-verb",
        "struct Storage[N: Usize] { erg values: Array[u32, N], } verb clear[N: Usize](ins buffer: Storage[N]) { buffer.values[0] = 42u32; } verb main() -> Int { erg buffer: Storage[4] = Storage[4] { values: Array[u32, 4](), }; clear(buffer: ins buffer); return buffer.values[0] as Int; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_const_generic_parameter_as_a_compile_time_expression_value() {
    let status = run_array_fixture(
        "const-generic-expression-value",
        "verb read_capacity[N: Usize]() -> u32 { return N as u32; } verb main() -> Int { erg value: u32 = read_capacity[4](); return value as Int; }",
    );
    assert_eq!(status.code(), Some(4));
}

#[cfg(unix)]
#[test]
fn specializes_const_generic_expression_values_from_struct_arguments() {
    let status = run_array_fixture(
        "const-generic-expression-from-struct",
        "struct Storage[N: Usize] { erg values: Array[u32, N], } verb capacity[N: Usize](ins storage: Storage[N]) -> u32 { if 0u32 < (N as u32) { return N as u32; } return 0u32; } verb main() -> Int { erg storage: Storage[4] = Storage[4] { values: Array[u32, 4](), }; return capacity(storage: ins storage) as Int; }",
    );
    assert_eq!(status.code(), Some(4));
}

#[cfg(unix)]
#[test]
fn accepts_const_generic_values_in_case_guards() {
    let status = run_array_fixture(
        "const-generic-case-guard",
        "verb choose[N: Usize]() -> u32 { return case true { true if 0u32 < (N as u32) => N as u32, _ => 0u32, }; } verb main() -> Int { return choose[4]() as Int; }",
    );
    assert_eq!(status.code(), Some(4));
}

#[cfg(unix)]
#[test]
fn specializes_transitive_const_generic_verb_calls_natively() {
    let status = run_array_fixture(
        "transitive-const-generic-verb",
        "struct Storage[N: Usize] { erg values: Array[u32, N], } verb inner[N: Usize](ins storage: Storage[N]) -> u32 { return N as u32; } verb outer[N: Usize](ins storage: Storage[N]) -> u32 { return inner(storage: ins storage); } verb main() -> Int { erg storage: Storage[4] = Storage[4] { values: Array[u32, 4](), }; return outer(storage: ins storage) as Int; }",
    );
    assert_eq!(status.code(), Some(4));
}

#[cfg(unix)]
#[test]
fn specializes_multiple_const_generic_instances_in_one_call_graph() {
    let status = run_array_fixture(
        "multiple-const-generic-instances",
        "verb read_capacity[N: Usize]() -> u32 { return N as u32; } verb main() -> Int { return read_capacity[4]() as Int + read_capacity[8]() as Int; }",
    );
    assert_eq!(status.code(), Some(12));
}

#[cfg(unix)]
#[test]
fn returns_transitive_aggregate_values_through_native_slots() {
    let status = run_array_fixture(
        "transitive-aggregate-return-slots",
        "struct Point { erg x: Int, } pack Frame { erg storage: Array[u8, 1]; layout little; fields { erg marker: u8 at 0; } } struct Box[N: Usize] { erg values: Array[u32, N], } verb make_point() -> Point { return Point { x: 41, }; } verb forward_point() -> Point { return make_point(); } verb make_frame() -> Frame { erg frame = Frame { storage: Array[u8, 1](), }; frame.marker = 41u8; return frame; } verb forward_frame() -> Frame { return make_frame(); } verb make_array() -> Array[u32, 2] { erg values: Array[u32, 2] = Array[u32, 2](); values[0] = 41u32; return values; } verb forward_array() -> Array[u32, 2] { return make_array(); } verb make_box[N: Usize]() -> Box[N] { erg box: Box[N] = Box[N] { values: Array[u32, N](), }; box.values[0] = 41u32; return box; } verb forward_box[N: Usize]() -> Box[N] { return make_box(); } verb main() -> Int { erg point = forward_point(); erg frame = forward_frame(); erg values: Array[u32, 2] = forward_array(); erg box: Box[2] = forward_box(); if point.x == 41 && frame.marker == 41u8 && values[0] == 41u32 && box.values[0] == 41u32 { return 0; } return 1; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn returns_an_array_through_the_native_return_slot() {
    let status = run_array_fixture(
        "array-return-slot",
        "verb make_values() -> Array[u32, 2] { erg values: Array[u32, 2] = Array[u32, 2](); values[0] = 41u32; values[1] = 42u32; return values; } verb main() -> Int { erg values: Array[u32, 2] = make_values(); if values[0] == 41u32 && values[1] == 42u32 { return 0; } return 1; }",
    );
    assert_eq!(status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn returns_an_eight_element_array_through_the_native_return_slot() {
    let status = run_array_fixture(
        "array-eight-return-slot",
        "verb forward_impulse() -> Array[u32, 8] { erg values: Array[u32, 8] = Array[u32, 8](); values[0] = 41u32; values[7] = 42u32; return values; } verb main() -> Int { erg values: Array[u32, 8] = forward_impulse(); return values[0] as Int + values[7] as Int - 83; }",
    );
    assert_eq!(status.code(), Some(0));
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
fn lowers_named_pack_offsets_natively() {
    let status = run_array_fixture(
        "named-pack-offset",
        "const BASE_OFFSET: u16 = 0u16; const MARKER_OFFSET: u16 = BASE_OFFSET + 8u16; pack Frame { erg storage: Array[u8, 2]; layout little; fields { erg prefix: u8 at BASE_OFFSET; erg marker: u8 at MARKER_OFFSET; } } verb main() -> Int { erg frame = Frame { storage: Array[u8, 2](), }; frame.marker = 41u8; return frame.marker as Int + 1; }",
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
fn executes_nested_if_expression_with_a_diverging_branch() {
    let status = run_array_fixture(
        "nested-if-diverging-branch",
        "verb choose(erg ready: Bool) -> Int { erg selected = if ready { if true { 41 } else { 42 } } else { return 0; }; return selected + 1; } verb main() -> Int { erg ready = true; return choose(ready: ready); }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_typed_case_branch_with_a_returning_branch() {
    let status = run_array_fixture(
        "typed-case-diverging-branch",
        "verb choose() -> Int { return case true { true => { return 41; }, _ => 0, }; } verb main() -> Int { return choose() + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_calls_across_nested_statement_blocks() {
    let status = run_array_fixture(
        "nested-call-statements",
        "verb append_one(ins bytes: Buffer) { append(bytes, 1u8); } verb worker() -> Int { erg bytes = Buffer[0]; if true { { append_one(bytes: ins bytes); } } return bytes[0] as Int + 41; } verb main() -> Int { return worker(); }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_calls_inside_loop_and_case_blocks() {
    let status = run_array_fixture(
        "nested-loop-case-call",
        "verb marker() -> Int { return 1; } verb main() -> Int { erg value = 0; loop { case value { 0 => { value = marker(); break; }, _ => { break; }, }; } return value + 41; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn propagates_try_from_a_nested_call_statement() {
    let status = run_array_fixture(
        "nested-call-try",
        "enum IoError { Failed, } verb produce() -> Result[Int, IoError] { return Ok(41); } verb worker() -> Result[Int, IoError] { if true { produce()?; } return Ok(41); } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(value) => value + 1, Result.Err(_) => 0, }; }",
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
fn executes_array_of_array_backed_packs_natively() {
    let status = run_array_fixture(
        "array-of-array-backed-packs",
        "pack Cell { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } verb main() -> Int { erg cells: Array[Cell, 2] = Array[Cell, 2](); cells[1] = Cell { storage: Array[u8, 2](), }; cells[1].marker = 41u8; return cells[1].marker as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn executes_compound_assignment_through_array_of_packs_natively() {
    let status = run_array_fixture(
        "array-of-packs-compound",
        "pack Cell { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } verb main() -> Int { erg cells: Array[Cell, 2] = Array[Cell, 2](); cells[1] = Cell { storage: Array[u8, 2](), }; cells[1].marker = 40u8; cells[1].marker += 1u8; return cells[1].marker as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn preserves_ins_loan_through_an_array_of_packs_natively() {
    let status = run_array_fixture(
        "array-of-packs-ins",
        "pack Cell { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } verb mutate(ins cell: Cell) { cell.marker = 41u8; } verb main() -> Int { erg cells: Array[Cell, 2] = Array[Cell, 2](); cells[1] = Cell { storage: Array[u8, 2](), }; mutate(cell: ins cells[1]); return cells[1].marker as Int + 1; }",
    );
    assert_eq!(status.code(), Some(42));
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
        "pack Frame { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } verb main() -> Int { erg frame = Frame { storage: Array[u8, 2](), }; frame.marker = 41u8; return frame.marker as Int + 1; }",
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

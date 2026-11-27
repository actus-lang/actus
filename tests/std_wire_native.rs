use std::fs;
use std::path::PathBuf;
use std::process::Command;

use object::{Object, ObjectSymbol};

fn compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_actus"))
}

#[test]
fn unused_wire_modules_are_absent_from_native_object_set() {
    let root = std::env::temp_dir().join(format!("actus-wire-reachability-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"wire_reachability\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("manifest");
    copy_standard_library(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        &root.join("src"),
    );
    fs::write(
        root.join("src/main.act"),
        "import { open std::wire };\n\"\"\"Entry point that returns the WIRE_VERSION constant as Int.\"\"\"\nverb main() -> Int { return WIRE_VERSION as Int; }\n",
    )
    .expect("source");
    let output = root.join("wire.o");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));

    let mut wire_symbols = Vec::new();
    for entry in fs::read_dir(&root).expect("read fixture directory") {
        let path = entry.expect("read fixture entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("o") {
            continue;
        }
        let bytes = fs::read(path).expect("read generated object");
        let file = object::File::parse(bytes.as_slice()).expect("parse generated object");
        wire_symbols.extend(file.symbols().filter(|symbol| !symbol.is_undefined()).filter_map(
            |symbol| {
                let name = symbol.name().ok()?.to_owned();
                name.contains("std_4_wire__verb_").then_some(name)
            },
        ));
    }
    assert!(wire_symbols.is_empty(), "unused Wire declarations were emitted: {wire_symbols:?}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(not(target_os = "macos"), feature = "cranelift"))]
#[test]
fn freestanding_wire_types_emit_without_host_runtime_imports() {
    let root = std::env::temp_dir().join(format!("actus-wire-freestanding-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"wire_freestanding\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"boot\"\n\n[build]\nruntime = \"std\"\ntarget = \"x86_64-unknown-uefi\"\nentry_contract = \"freestanding\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    copy_standard_library(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        &root.join("src"),
    );
    fs::write(
        root.join("src/main.act"),
        "import { open std::wire };\n\"\"\"Boot verb that builds a WireHeader, sets its version field to WIRE_VERSION and returns the field as Int.\"\"\"\nverb boot() -> Int { erg header = WireHeader { storage: Array[u8, 12]() }; header.version = WIRE_VERSION; return header.version as Int; }\n",
    )
    .expect("source");
    let output = root.join("wire.o");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let bytes = fs::read(output).expect("read object");
    let file = object::File::parse(bytes.as_slice()).expect("parse object");
    let undefined = file
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name().ok())
        .filter(|name| !name.contains("std_4_wire__verb_") && *name != "actus_buffer_drop")
        .collect::<Vec<_>>();
    assert!(undefined.is_empty(), "freestanding Wire object imports host symbols: {undefined:?}");
    let _ = fs::remove_dir_all(root);
}

fn copy_standard_library(source: &std::path::Path, destination: &std::path::Path) {
    fs::create_dir_all(destination).expect("create standard-library directory");
    for entry in fs::read_dir(source).expect("read standard-library directory") {
        let entry = entry.expect("read standard-library entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_standard_library(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy standard-library source");
        }
    }
}

#[test]
fn hosted_wire_crc16_executes_with_deterministic_reference_value() {
    let root = std::env::temp_dir().join(format!("actus-wire-native-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"wire_native\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    copy_standard_library(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        &root.join("src"),
    );
    fs::write(
        root.join("src/main.act"),
        r#"import { open std::wire };
"""
contract:
purpose:
    Compute the CRC-16 CCITT checksum of the bytes 1, 2 and 3 and check that it equals 44461.
outputs:
    0 when the checksum matches, 1 when it differs, 2 when it cannot be computed.
ownership:
    The input buffer is borrowed by the checksum call.
errors:
    A WireError from wire_crc16_ccitt maps to 2.
"""
verb crc_status() -> Int {
    erg crc_input: Buffer = Buffer[0];
    append(crc_input, 1u8);
    append(crc_input, 2u8);
    append(crc_input, 3u8);
    erg start: u32 = 0u32;
    erg end: u32 = 3u32;
    erg checksum = wire_crc16_ccitt(input: abs crc_input, start: erg start, end: erg end);
    return case dat checksum {
        Result.Ok(value) => if value == 44461u16 { 0 } else { 1 },
        Result.Err(_) => 2,
    };
}
"""
contract:
purpose:
    Fill a WireHeader field by field, check that a 12-byte output is too small for the frame, and then run the full frame round trip.
outputs:
    0 on success, 5 when the capacity check fails, otherwise the status of frame_status.
ownership:
    The header is moved into frame_status after being borrowed by capacity_status.
"""
verb header_status() -> Int {
    erg header = WireHeader {storage: Array[u8, 12]()};
    header.magic_0 = WIRE_MAGIC_0;
    header.magic_1 = WIRE_MAGIC_1;
    header.version = WIRE_VERSION;
    header.flags = WIRE_FLAG_ACK_REQUIRED;
    header.message_id = 9u8;
    header.channel_id = 3u8;
    header.sequence_num = 287454020u32;
    header.payload_len = 3u16;
    erg payload: Buffer = Buffer[0];
    append(payload, 7u8);
    append(payload, 8u8);
    append(payload, 9u8);
    if capacity_status(header: abs header, payload: abs payload) != 0 {
        return 5;
    }
    return frame_status(header: dat header);
}
"""
contract:
purpose:
    Try to encode a frame into a 12-byte output buffer and check that it fails with the insufficient capacity error.
inputs:
    header: borrowed WireHeader; payload: borrowed Buffer.
outputs:
    0 for WireError.InsufficientCapacity, 1 when the encode succeeds, 2 for another error.
ownership:
    Header and payload are borrowed; the output is local.
errors:
    Only the insufficient capacity error counts as the expected outcome.
"""
verb capacity_status(abs header: WireHeader, abs payload: Buffer) -> Int {
    erg output = Buffer[12];
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat result {
        Result.Ok(_) => 1,
        Result.Err(error) => case dat error {
            WireError.InsufficientCapacity => 0,
            _ => 2,
        },
    };
}
"""
contract:
purpose:
    Encode a frame with the payload 7, 8, 9, decode it again, compare the sequence number and payload, then corrupt one checksum byte and continue with bad_checksum_status.
inputs:
    header: owned WireHeader to encode.
outputs:
    0 on success, 3 when the encode fails, 4 when the decode fails, 1 when the round trip differs, otherwise the status of bad_checksum_status.
ownership:
    The header is borrowed by the encode call; the encoded buffer is mutated and passed on.
errors:
    WireError values from encode and decode map to 3 and 4.
"""
verb frame_status(dat header: WireHeader) -> Int {
    erg payload: Buffer = Buffer[0];
    append(payload, 7u8);
    append(payload, 8u8);
    append(payload, 9u8);
    erg encoded = Buffer[17];
    erg encoded_result = wire_frame_encode(header: abs header, payload: abs payload, output: ins encoded);
    return case dat encoded_result {
        Result.Err(_) => 3,
        Result.Ok(count) => {
            erg decoded_payload = Buffer[3];
            erg decoded = wire_frame_decode(input: abs encoded, payload_output: ins decoded_payload);
            return case dat decoded {
                Result.Err(_) => 4,
                Result.Ok(value) => {
                    if count == 17u32 && value.sequence_num == 287454020u32 && decoded_payload[0] == 7u8 && decoded_payload[2] == 9u8 {
                        encoded[12] ^= 1u8;
                        return bad_checksum_status(encoded: ins encoded);
                    }
                    return 1;
                },
            };
        },
    };
}
"""
contract:
purpose:
    Decode a frame whose checksum byte was corrupted and check that it fails with the checksum mismatch error, leaving the payload output untouched.
inputs:
    encoded: exclusive ins loan of Buffer holding the corrupted frame.
outputs:
    0 when the checksum mismatch is reported and the payload output keeps 91, 92, 93, 1 for another error, 2 when decoding succeeds, 3 when the output was modified.
ownership:
    The encoded buffer is an exclusive ins loan; the payload output is local.
errors:
    Only the checksum mismatch error counts as the expected outcome.
"""
verb bad_checksum_status(ins encoded: Buffer) -> Int {
    erg payload: Buffer = Buffer[3];
    payload[0] = 91u8;
    payload[1] = 92u8;
    payload[2] = 93u8;
    erg decoded = wire_frame_decode(input: abs encoded, payload_output: ins payload);
    return case dat decoded {
        Result.Ok(_) => 2,
        Result.Err(error) => case dat error {
            WireError.ChecksumMismatch => if payload[0] == 91u8 && payload[1] == 92u8 && payload[2] == 93u8 { 0 } else { 3 },
            _ => 1,
        },
    };
}
"""
contract:
purpose:
    Decode a header whose first magic byte is wrong and check that it fails with the invalid magic error.
outputs:
    0 for WireError.InvalidMagic, 1 for anything else.
ownership:
    The input buffer is borrowed by the decode call.
errors:
    Only the invalid magic error counts as the expected outcome.
"""
verb invalid_magic_status() -> Int {
    erg input: Buffer = Buffer[12];
    input[0] = 0u8;
    input[1] = WIRE_MAGIC_1;
    input[2] = WIRE_VERSION;
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.InvalidMagic => 0, _ => 1 }, _ => 1 };
}
"""
contract:
purpose:
    Decode a header with an unsupported version byte and check that it fails with the unsupported version error.
outputs:
    0 for WireError.UnsupportedVersion, 1 for anything else.
ownership:
    The input buffer is borrowed by the decode call.
errors:
    Only the unsupported version error counts as the expected outcome.
"""
verb invalid_version_status() -> Int {
    erg input: Buffer = Buffer[12];
    input[0] = WIRE_MAGIC_0;
    input[1] = WIRE_MAGIC_1;
    input[2] = 2u8;
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.UnsupportedVersion => 0, _ => 1 }, _ => 1 };
}
"""
contract:
purpose:
    Decode a header with an unknown flag bit set and check that it fails with the unknown flags error.
outputs:
    0 for WireError.UnknownFlags, 1 for anything else.
ownership:
    The input buffer is borrowed by the decode call.
errors:
    Only the unknown flags error counts as the expected outcome.
"""
verb unknown_flags_status() -> Int {
    erg input: Buffer = Buffer[12];
    input[0] = WIRE_MAGIC_0;
    input[1] = WIRE_MAGIC_1;
    input[2] = WIRE_VERSION;
    input[3] = 1u8;
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.UnknownFlags => 0, _ => 1 }, _ => 1 };
}
"""
contract:
purpose:
    Decode a header that declares a payload length of 1025 and check that it fails with the payload too large error.
outputs:
    0 for WireError.PayloadTooLarge, 1 to 4 for other specific errors, 5 for any other error, 6 when decoding succeeds.
ownership:
    The input buffer is borrowed by the decode call.
errors:
    Only the payload too large error counts as the expected outcome.
"""
verb oversized_header_status() -> Int {
    erg input: Buffer = Buffer[12];
    input[0] = WIRE_MAGIC_0;
    input[1] = WIRE_MAGIC_1;
    input[2] = WIRE_VERSION;
    input[3] = 0u8;
    input[10] = 1u8;
    input[11] = 4u8;
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded {
        Result.Err(error) => case dat error {
            WireError.InvalidMagic => 1,
            WireError.UnsupportedVersion => 2,
            WireError.UnknownFlags => 3,
            WireError.PayloadTooLarge => 0,
            WireError.Truncated => 4,
            _ => 5,
        },
        Result.Ok(_) => 6,
    };
}
"""
contract:
purpose:
    Decode a header from an 11-byte buffer, one byte shorter than a header, and check that it fails with the truncated error.
outputs:
    0 for WireError.Truncated, 1 for anything else.
ownership:
    The input buffer is borrowed by the decode call.
errors:
    Only the truncated error counts as the expected outcome.
"""
verb truncated_header_status() -> Int {
    erg input: Buffer = Buffer[11];
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.Truncated => 0, _ => 1 }, _ => 1 };
}
"""
contract:
purpose:
    Encode a frame into a 1038-byte output and check that the reported byte count equals the expected one.
inputs:
    header: borrowed WireHeader; payload: borrowed Buffer; expected_count: borrowed u32 frame length.
outputs:
    0 when the count matches, 1 when it differs, 2 when the encode fails.
ownership:
    All three parameters are borrowed; the output is local.
errors:
    A WireError from wire_frame_encode maps to 2.
"""
verb frame_count_status(abs header: WireHeader, abs payload: Buffer, abs expected_count: u32) -> Int {
    erg output = Buffer[1038];
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat result {
        Result.Ok(count) => if count == expected_count { 0 } else { 1 },
        Result.Err(_) => 2,
    };
}
"""
contract:
purpose:
    Encode a frame with a 1025-byte payload and check that it fails with the payload too large error.
inputs:
    header: borrowed WireHeader declaring the oversized payload.
outputs:
    0 for WireError.PayloadTooLarge, 1 for another error, 2 when the encode succeeds.
ownership:
    The header is borrowed; payload and output buffers are local.
errors:
    Only the payload too large error counts as the expected outcome.
"""
verb oversized_status(abs header: WireHeader) -> Int {
    erg payload: Buffer = Buffer[1025];
    erg output = Buffer[1039];
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat result {
        Result.Err(error) => case dat error { WireError.PayloadTooLarge => 0, _ => 1 },
        Result.Ok(_) => 2,
    };
}
"""Encodes a frame with the maximum 1024-byte payload and checks that its length is 1038 bytes, returning the status of frame_count_status."""
verb maximum_status() -> Int {
    erg maximum_header = WireHeader {storage: Array[u8, 12]()};
    maximum_header.magic_0 = WIRE_MAGIC_0;
    maximum_header.magic_1 = WIRE_MAGIC_1;
    maximum_header.version = WIRE_VERSION;
    maximum_header.payload_len = 1024u16;
    erg maximum_payload: Buffer = Buffer[1024];
    erg expected_count: u32 = 1038u32;
    return frame_count_status(header: abs maximum_header, payload: abs maximum_payload, expected_count: abs expected_count);
}
"""Encodes a frame with an empty payload and checks that its length is 14 bytes, returning the status of frame_count_status."""
verb empty_status() -> Int {
    erg empty_header = WireHeader {storage: Array[u8, 12]()};
    empty_header.magic_0 = WIRE_MAGIC_0;
    empty_header.magic_1 = WIRE_MAGIC_1;
    empty_header.version = WIRE_VERSION;
    empty_header.payload_len = 0u16;
    erg empty_payload: Buffer = Buffer[0];
    erg empty_count: u32 = 14u32;
    return frame_count_status(header: abs empty_header, payload: abs empty_payload, expected_count: abs empty_count);
}
"""Encodes a frame with a one-byte payload and checks that its length is 15 bytes, returning the status of frame_count_status."""
verb minimum_status() -> Int {
    erg minimum_header = WireHeader {storage: Array[u8, 12]()};
    minimum_header.magic_0 = WIRE_MAGIC_0;
    minimum_header.magic_1 = WIRE_MAGIC_1;
    minimum_header.version = WIRE_VERSION;
    minimum_header.payload_len = 1u16;
    erg minimum_payload: Buffer = Buffer[1];
    erg minimum_count: u32 = 15u32;
    return frame_count_status(header: abs minimum_header, payload: abs minimum_payload, expected_count: abs minimum_count);
}
"""Runs the empty, one-byte and maximum payload checks, then the oversized payload check, and returns 0 when all pass or the number 1 to 4 of the first failing check."""
verb boundary_status() -> Int {
    erg empty = empty_status();
    if empty != 0 {
        return 1;
    }
    erg minimum = minimum_status();
    if minimum != 0 {
        return 2;
    }
    erg maximum = maximum_status();
    if maximum != 0 {
        return 3;
    }
    erg oversized_header = WireHeader {storage: Array[u8, 12]()};
    oversized_header.magic_0 = WIRE_MAGIC_0;
    oversized_header.magic_1 = WIRE_MAGIC_1;
    oversized_header.version = WIRE_VERSION;
    oversized_header.payload_len = 1025u16;
    if oversized_status(header: abs oversized_header) != 0 {
        return 4;
    }
    return 0;
}
"""Compares the first length bytes of two buffers one by one and returns false at the first difference, true otherwise."""
verb buffers_equal(abs left: Buffer, abs right: Buffer, abs length: u32) -> Bool {
    for erg index: u32 in 0u32 .. length {
        erg left_byte: u8 = left[index];
        erg right_byte: u8 = right[index];
        if left_byte != right_byte {
            return false;
        }
    }
    return true;
}
"""
contract:
purpose:
    Encode a frame into the given output buffer and report only success or failure.
inputs:
    header: borrowed WireHeader; payload: borrowed Buffer; output: exclusive ins loan of Buffer receiving the frame.
outputs:
    0 when the encode succeeds, 1 when it fails.
ownership:
    The header and payload are borrowed; the output is an exclusive ins loan that is written.
errors:
    A WireError from wire_frame_encode maps to 1.
side_effects:
    Writes the frame into the output buffer.
"""
verb deterministic_encode_status(abs header: WireHeader, abs payload: Buffer, ins output: Buffer) -> Int {
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat result {
        Result.Ok(_) => 0,
        Result.Err(_) => 1,
    };
}
"""
contract:
purpose:
    Encode the same frame twice into two separate buffers and check that the 18 bytes are identical.
outputs:
    0 when both encodes succeed and the frames are equal, 1 or 2 when an encode fails, 3 when the frames differ.
ownership:
    The header and payload are borrowed; both outputs are local and compared afterwards.
errors:
    Encode failures map to 1 and 2.
side_effects:
    Writes the two frames into local buffers.
"""
verb deterministic_status() -> Int {
    erg header = WireHeader {storage: Array[u8, 12]()};
    header.magic_0 = WIRE_MAGIC_0;
    header.magic_1 = WIRE_MAGIC_1;
    header.version = WIRE_VERSION;
    header.payload_len = 4u16;
    erg payload: Buffer = Buffer[0];
    append(payload, 3u8);
    append(payload, 1u8);
    append(payload, 4u8);
    append(payload, 1u8);
    erg first = Buffer[18];
    erg second = Buffer[18];
    if deterministic_encode_status(header: abs header, payload: abs payload, output: ins first) != 0 {
        return 1;
    }
    if deterministic_encode_status(header: abs header, payload: abs payload, output: ins second) != 0 {
        return 2;
    }
    erg frame_length: u32 = 18u32;
    if !buffers_equal(left: abs first, right: abs second, length: abs frame_length) {
        return 3;
    }
    return 0;
}
"""Runs the invalid magic, invalid version, unknown flags, oversized header and truncated header checks in order and returns 0 when all pass or the number 1 to 5 of the first failing check."""
verb negative_status() -> Int {
    if invalid_magic_status() != 0 {
        return 1;
    }
    if invalid_version_status() != 0 {
        return 2;
    }
    if unknown_flags_status() != 0 {
        return 3;
    }
    if oversized_header_status() != 0 {
        return 4;
    }
    if truncated_header_status() != 0 {
        return 5;
    }
    return 0;
}
"""Entry point that runs the checksum, header, negative, boundary and deterministic encoding checks, and returns 0 when all pass or the code 7 to 11 of the first failing group."""
verb main() -> Int {
    if crc_status() != 0 {
        return 7;
    }
    if header_status() != 0 {
        return 8;
    }
    erg negative = negative_status();
    if negative != 0 {
        return 11;
    }
    erg boundary = boundary_status();
    if boundary != 0 {
        return 9;
    }
    if deterministic_status() != 0 {
        return 10;
    }
    return 0;
}
"#,
    )
    .expect("source");
    let output = root.join("wire-native");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let execution = Command::new(&output).output().expect("execute");
    assert_eq!(
        execution.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&execution.stdout),
        String::from_utf8_lossy(&execution.stderr)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_wire_endpoint_parser_is_optional_and_bounded() {
    let root = std::env::temp_dir().join(format!("actus-wire-endpoint-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"wire_endpoint_native\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    copy_standard_library(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        &root.join("src"),
    );
    fs::write(
        root.join("src/main.act"),
        r#"import { open std::wire };
"""
contract:
purpose:
    Parse the endpoint text wire://robot:8080/c... and check that the port is 8080, a port is present and the path length is 8.
outputs:
    0 when the endpoint parses as expected, 2 when a field differs, 15 or 16 or another code for the specific WireEndpointError.
ownership:
    The input buffer is borrowed by the parse call.
errors:
    Each WireEndpointError maps to its own exit code.
"""
verb valid_status() -> Int {
    erg input = Buffer[25];
    input[0] = 119u8; input[1] = 105u8; input[2] = 114u8; input[3] = 101u8;
    input[4] = 58u8; input[5] = 47u8; input[6] = 47u8;
    input[7] = 114u8; input[8] = 111u8; input[9] = 98u8; input[10] = 111u8; input[11] = 116u8;
    input[12] = 58u8; input[13] = 56u8; input[14] = 48u8; input[15] = 56u8; input[16] = 48u8;
    input[17] = 47u8; input[18] = 99u8; input[19] = 111u8; input[20] = 110u8; input[21] = 116u8; input[22] = 114u8; input[23] = 111u8; input[24] = 108u8;
    erg parsed = wire_endpoint_parse(input: abs input);
    return case dat parsed {
        Result.Err(error) => case dat error {
            WireEndpointError.TooLong => 10,
            WireEndpointError.InvalidScheme => 11,
            WireEndpointError.MissingAuthority => 12,
            WireEndpointError.InvalidAuthority => 13,
            WireEndpointError.InvalidPort => 14,
            WireEndpointError.PortOutOfRange => 15,
            WireEndpointError.InvalidPath => 16,
        },
        Result.Ok(endpoint) => if endpoint.port == 8080u32 && endpoint.has_port == 1u32 && endpoint.path_length == 8u32 { 0 } else { 2 },
    };
}
"""
contract:
purpose:
    Parse the text wire:// with no authority and check that it fails with the missing authority error.
outputs:
    0 for WireEndpointError.MissingAuthority, 1 for another error, 2 when the parse succeeds.
ownership:
    The input buffer is borrowed by the parse call.
errors:
    Only the missing authority error counts as the expected outcome.
"""
verb invalid_status() -> Int {
    erg input = Buffer[7];
    input[0] = 119u8; input[1] = 105u8; input[2] = 114u8; input[3] = 101u8;
    input[4] = 58u8; input[5] = 47u8; input[6] = 47u8;
    erg parsed = wire_endpoint_parse(input: abs input);
    return case dat parsed { Result.Err(error) => case dat error { WireEndpointError.MissingAuthority => 0, _ => 1, }, Result.Ok(_) => 2, };
}
"""Entry point that runs the valid and the invalid endpoint checks and returns the status of the valid one."""
verb main() -> Int {
    erg valid = valid_status();
    erg invalid = invalid_status();
    return valid;
}
"#,
    )
    .expect("source");
    let output = root.join("wire-endpoint");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let execution = Command::new(&output).output().expect("execute");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_wire_parser_accepts_split_frame_chunks() {
    let root = std::env::temp_dir().join(format!("actus-wire-parser-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"wire_parser\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    copy_standard_library(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        &root.join("src"),
    );
    fs::write(
        root.join("src/main.act"),
        r#"import { open std::wire };
"""
contract:
purpose:
    Feed an encoded 17-byte frame to the parser in two chunks of 5 and 12 bytes and check that it is Collecting after the first and Ready after the second, with the right header and payload.
inputs:
    encoded: borrowed Buffer holding the frame.
outputs:
    0 on success, 1 or 3 when a feed fails, 2 or 4 when a count or state is wrong, 5 when the header or payload differ.
ownership:
    The encoded buffer is borrowed and copied into two chunk buffers; the parser is local.
errors:
    A WireError from wire_parser_feed maps to 1 and 3.
"""
verb feed_chunks(abs encoded: Buffer) -> Int {
    erg first: Buffer = Buffer[0];
    for erg index: u32 in 0u32 .. 5u32 { append(first, encoded[index]); }
    erg second: Buffer = Buffer[0];
    for erg index: u32 in 5u32 .. 17u32 { append(second, encoded[index]); }
    erg parser = wire_parser_empty();
    erg payload: Buffer = Buffer[3];
    payload[0] = 91u8; payload[1] = 92u8; payload[2] = 93u8;
    erg first_result = wire_parser_feed(parser: ins parser, chunk: abs first, payload_output: ins payload);
    erg first_count: u32 = case dat first_result { Result.Err(_) => { return 1; }, Result.Ok(count) => count, };
    erg first_status = wire_parser_status(parser: abs parser);
    erg first_state: Int = case dat first_status { WireParserStatus.Collecting => 0, _ => 1, };
    if first_count != 5u32 || first_state != 0 || payload[0] != 91u8 { return 2; }
    erg second_result = wire_parser_feed(parser: ins parser, chunk: abs second, payload_output: ins payload);
    erg second_count: u32 = case dat second_result { Result.Err(_) => { return 3; }, Result.Ok(count) => count, };
    erg second_status = wire_parser_status(parser: abs parser);
    erg second_state: Int = case dat second_status { WireParserStatus.Ready => 0, _ => 1, };
    if second_count != 12u32 || second_state != 0 { return 4; }
    erg parsed = wire_parser_header(parser: abs parser);
    if parsed.sequence_num != 287454020u32 || payload[0] != 7u8 || payload[2] != 9u8 { return 5; }
    return 0;
}
"""
contract:
purpose:
    Feed four noise bytes to an empty parser and check that all four are consumed while it stays in the Searching state.
outputs:
    0 on success, 1 when the feed fails, 2 when the consumed count is wrong, 3 when the state is not Searching.
ownership:
    The noise buffer is borrowed by the feed call; the parser is local.
errors:
    A WireError from wire_parser_feed maps to 1.
"""
verb parser_noise_status() -> Int {
    erg noise: Buffer = Buffer[0];
    append(noise, 0u8); append(noise, 1u8); append(noise, 2u8); append(noise, 3u8);
    erg parser = wire_parser_empty();
    erg output: Buffer = Buffer[3];
    erg result = wire_parser_feed(parser: ins parser, chunk: abs noise, payload_output: ins output);
    erg consumed: u32 = case dat result { Result.Err(_) => { return 1; }, Result.Ok(count) => count, };
    if consumed != 4u32 { return 2; }
    erg status = wire_parser_status(parser: abs parser);
    return case dat status { WireParserStatus.Searching => 0, _ => 3, };
}
"""
contract:
purpose:
    Feed a frame preceded by one junk byte to the parser and check that it resynchronizes on the magic bytes, consumes 18 bytes and becomes Ready with the first payload byte 7.
inputs:
    encoded: borrowed Buffer holding a valid frame.
outputs:
    0 on success, 1 when the feed fails, 2 when the count, payload or state is wrong.
ownership:
    The encoded buffer is borrowed and copied behind the junk byte; the parser is local.
errors:
    A WireError from wire_parser_feed maps to 1.
"""
verb parser_magic_resync_status(abs encoded: Buffer) -> Int {
    erg input: Buffer = Buffer[18];
    input[0] = 0u8;
    for erg index: u32 in 0u32 .. 17u32 { input[index + 1u32] = encoded[index]; }
    erg parser = wire_parser_empty();
    erg output: Buffer = Buffer[3];
    erg result = wire_parser_feed(parser: ins parser, chunk: abs input, payload_output: ins output);
    return case dat result {
        Result.Err(_) => 1,
        Result.Ok(count) => {
            erg status = wire_parser_status(parser: abs parser);
            erg ready: Bool = case dat status { WireParserStatus.Ready => true, _ => false, };
            if count == 18u32 && output[0] == 7u8 && ready { return 0; }
            return 2;
        },
    };
}
"""
contract:
purpose:
    Set an unknown flag bit in the encoded frame, feed it to the parser, check that it fails with the unknown flags error and returns to Searching, then restore the byte.
inputs:
    encoded: exclusive ins loan of Buffer holding a valid frame that is modified temporarily.
outputs:
    0 on success, 1 or 2 for a wrong result, 3 when the parser is not Searching.
ownership:
    The encoded buffer is an exclusive ins loan that is changed and restored.
errors:
    Only the unknown flags error counts as the expected outcome.
side_effects:
    Temporarily changes byte 3 of the encoded buffer.
"""
verb parser_flags_status(ins encoded: Buffer) -> Int {
    encoded[3] = 1u8;
    erg parser = wire_parser_empty();
    erg output: Buffer = Buffer[3];
    erg result = wire_parser_feed(parser: ins parser, chunk: abs encoded, payload_output: ins output);
    encoded[3] = 0u8;
    erg code: Int = case dat result { Result.Err(error) => case dat error { WireError.UnknownFlags => 0, _ => 1, }, Result.Ok(_) => 2, };
    if code != 0 { return code; }
    erg status = wire_parser_status(parser: abs parser);
    return case dat status { WireParserStatus.Searching => 0, _ => 3, };
}
"""
contract:
purpose:
    Set an oversized payload length in the encoded frame, feed it to the parser, check that it fails with the payload too large error and returns to Searching, then restore the bytes.
inputs:
    encoded: exclusive ins loan of Buffer holding a valid frame that is modified temporarily.
outputs:
    0 on success, 1 or 2 for a wrong result, 3 when the parser is not Searching.
ownership:
    The encoded buffer is an exclusive ins loan that is changed and restored.
errors:
    Only the payload too large error counts as the expected outcome.
side_effects:
    Temporarily changes bytes 10 and 11 of the encoded buffer.
"""
verb parser_length_status(ins encoded: Buffer) -> Int {
    encoded[10] = 1u8;
    encoded[11] = 4u8;
    erg parser = wire_parser_empty();
    erg output: Buffer = Buffer[3];
    erg result = wire_parser_feed(parser: ins parser, chunk: abs encoded, payload_output: ins output);
    encoded[10] = 3u8;
    encoded[11] = 0u8;
    erg code: Int = case dat result { Result.Err(error) => case dat error { WireError.PayloadTooLarge => 0, _ => 1, }, Result.Ok(_) => 2, };
    if code != 0 { return code; }
    erg status = wire_parser_status(parser: abs parser);
    return case dat status { WireParserStatus.Searching => 0, _ => 3, };
}
"""
contract:
purpose:
    Feed the parser a frame with an unsupported version and then a frame with a corrupted checksum, check each error and that the parser returns to Searching, then run the flags and length rejection checks.
inputs:
    encoded: exclusive ins loan of Buffer holding a valid frame that is modified temporarily.
outputs:
    0 on success, otherwise 1 to 6 for the first failing check.
ownership:
    The encoded buffer is an exclusive ins loan that is changed and restored between steps.
errors:
    Only the unsupported version and checksum mismatch errors count as the expected outcomes.
side_effects:
    Temporarily changes bytes 2 and 12 of the encoded buffer.
"""
verb parser_rejection_status(ins encoded: Buffer) -> Int {
    encoded[2] = 2u8;
    erg parser = wire_parser_empty();
    erg output: Buffer = Buffer[3];
    erg version_result = wire_parser_feed(parser: ins parser, chunk: abs encoded, payload_output: ins output);
    erg version_code: Int = case dat version_result {
        Result.Err(error) => case dat error { WireError.UnsupportedVersion => 0, _ => 1, },
        Result.Ok(_) => 2,
    };
    if version_code != 0 { return 1; }
    erg status = wire_parser_status(parser: abs parser);
    if case dat status { WireParserStatus.Searching => false, _ => true, } { return 2; }
    encoded[2] = WIRE_VERSION;
    encoded[12] ^= 1u8;
    erg checksum_result = wire_parser_feed(parser: ins parser, chunk: abs encoded, payload_output: ins output);
    erg checksum_code: Int = case dat checksum_result {
        Result.Err(error) => case dat error { WireError.ChecksumMismatch => 0, _ => 1, },
        Result.Ok(_) => 2,
    };
    encoded[12] ^= 1u8;
    if checksum_code != 0 { return 3; }
    erg checksum_status = wire_parser_status(parser: abs parser);
    if case dat checksum_status { WireParserStatus.Searching => false, _ => true, } { return 6; }
    if parser_flags_status(encoded: ins encoded) != 0 { return 4; }
    if parser_length_status(encoded: ins encoded) != 0 { return 5; }
    return 0;
}
"""Entry point that encodes a frame with the payload 7, 8, 9, then runs the parser noise, resynchronization, rejection and chunked feed checks on it, returning 0 when all pass, 6 when the encode fails and 7 when a parser check fails."""
verb main() -> Int {
    erg header = WireHeader {storage: Array[u8, 12]()};
    header.magic_0 = WIRE_MAGIC_0;
    header.magic_1 = WIRE_MAGIC_1;
    header.version = WIRE_VERSION;
    header.sequence_num = 287454020u32;
    header.payload_len = 3u16;
    erg payload: Buffer = Buffer[0];
    append(payload, 7u8); append(payload, 8u8); append(payload, 9u8);
    erg encoded = Buffer[17];
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins encoded);
    return case dat result { Result.Err(_) => 6, Result.Ok(_) => if parser_noise_status() == 0 && parser_magic_resync_status(encoded: abs encoded) == 0 && parser_rejection_status(encoded: ins encoded) == 0 { feed_chunks(encoded: abs encoded) } else { 7 }, };
}
"#,
    )
    .expect("source");
    let output = root.join("wire-parser");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let execution = Command::new(&output).output().expect("execute");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_wire_sequence_window_rejects_replays_and_stale_contexts() {
    let root = std::env::temp_dir().join(format!("actus-wire-sequence-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"wire_sequence\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    copy_standard_library(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        &root.join("src"),
    );
    fs::write(
        root.join("src/main.act"),
        r#"import { open std::wire };
"""
contract:
purpose:
    Accept a sequence number in a sequence window and report only success or failure.
inputs:
    window: exclusive ins loan of WireSequenceWindow; context_id: owned u32 context; sequence_num: owned u32 sequence number.
outputs:
    0 when the number is accepted, 1 when it is rejected.
ownership:
    The window is an exclusive ins loan that is updated on acceptance.
errors:
    Any WireError maps to 1.
side_effects:
    Updates the window on acceptance.
"""
verb accept_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Ok(_) => 0, Result.Err(_) => 1, };
}

"""
contract:
purpose:
    Offer a sequence number to the window and check that it is rejected as a duplicate.
inputs:
    window: exclusive ins loan of WireSequenceWindow; context_id: owned u32 context; sequence_num: owned u32 sequence number.
outputs:
    0 for WireError.SequenceDuplicate, 1 for another error, 2 when accepted.
ownership:
    The window is an exclusive ins loan.
errors:
    Only the duplicate error counts as the expected outcome.
"""
verb duplicate_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceDuplicate => 0, _ => 1, }, Result.Ok(_) => 2, };
}
"""
contract:
purpose:
    Offer a sequence number to the window and check that it is rejected as stale.
inputs:
    window: exclusive ins loan of WireSequenceWindow; context_id: owned u32 context; sequence_num: owned u32 sequence number.
outputs:
    0 for WireError.SequenceStale, 1 for another error, 2 when accepted.
ownership:
    The window is an exclusive ins loan.
errors:
    Only the stale error counts as the expected outcome.
"""
verb stale_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceStale => 0, _ => 1, }, Result.Ok(_) => 2, };
}
"""
contract:
purpose:
    Offer a sequence number to the window and check that it is rejected as a jump that is too large.
inputs:
    window: exclusive ins loan of WireSequenceWindow; context_id: owned u32 context; sequence_num: owned u32 sequence number.
outputs:
    0 for WireError.SequenceJumpTooLarge, 1 for another error, 2 when accepted.
ownership:
    The window is an exclusive ins loan.
errors:
    Only the jump too large error counts as the expected outcome.
"""
verb jump_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceJumpTooLarge => 0, _ => 1, }, Result.Ok(_) => 2, };
}
"""
contract:
purpose:
    Offer a sequence number for another context to the window and check that it is rejected as a context mismatch.
inputs:
    window: exclusive ins loan of WireSequenceWindow; context_id: owned u32 context; sequence_num: owned u32 sequence number.
outputs:
    0 for WireError.SequenceContextMismatch, 1 for another error, 2 when accepted.
ownership:
    The window is an exclusive ins loan.
errors:
    Only the context mismatch error counts as the expected outcome.
"""
verb context_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceContextMismatch => 0, _ => 1, }, Result.Ok(_) => 2, };
}
"""Drives a sequence window through numbers 10, 12, 11, a duplicate, 80, a stale 1 and a jump to 1105, checking the accepted or rejected outcome and the highest number after each step, and returns 0 or the number 1 to 9 of the first failing step."""
verb order_status() -> Int {
    erg window = wire_sequence_empty();
    erg context_id: u32 = 7u32;
    erg sequence_num: u32 = 10u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 1; }
    sequence_num = 12u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 2; }
    sequence_num = 11u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 3; }
    if duplicate_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 4; }
    sequence_num = 80u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 5; }
    sequence_num = 1u32;
    if stale_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 6; }
    if wire_sequence_highest(window: abs window) != 80u32 { return 7; }
    sequence_num = 1105u32;
    if jump_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 8; }
    if wire_sequence_highest(window: abs window) != 80u32 { return 9; }
    return 0;
}
"""Drives a sequence window through context changes, replacement, reset and the wrap around from 4294967295 to 0, checking the outcome after each step, and returns 0 or the code 10 to 15 of the first failing step."""
verb lifecycle_status() -> Int {
    erg window = wire_sequence_empty();
    erg context_id: u32 = 7u32;
    erg sequence_num: u32 = 0u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 10; }
    context_id = 8u32;
    sequence_num = 81u32;
    if context_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 10; }
    wire_sequence_replace_context(window: ins window, context_id: erg context_id);
    sequence_num = 0u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 11; }
    wire_sequence_reset(window: ins window);
    context_id = 9u32;
    sequence_num = 4294967294u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 12; }
    sequence_num = 4294967295u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 13; }
    sequence_num = 0u32;
    if accept_status(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num) != 0 { return 14; }
    return if wire_sequence_highest(window: abs window) == 0u32 { 0 } else { 15 };
}
"""Entry point that runs the sequence order check and the context lifecycle check, returning 0 when both pass or 1 or 2 for the first failing one."""
verb main() -> Int {
    if order_status() != 0 { return 1; }
    if lifecycle_status() != 0 { return 2; }
    return 0;
}
"#,
    )
    .expect("source");
    let output = root.join("wire-sequence");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let execution = Command::new(&output).output().expect("execute");
    assert_eq!(
        execution.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&execution.stdout),
        String::from_utf8_lossy(&execution.stderr)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_wire_fragment_reassembly_executes_bounded_lifecycle() {
    let root = std::env::temp_dir().join(format!("actus-wire-fragment-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"wire_fragment_native\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    copy_standard_library(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        &root.join("src"),
    );
    fs::write(
        root.join("src/main.act"),
        r#"import { open std::wire };
"""
contract:
purpose:
    Encode a three-byte payload as a fragment at the given offset and index into an output buffer.
inputs:
    output: exclusive ins loan of Buffer receiving the fragment; payload: borrowed Buffer; offset: owned u32 byte offset; index: owned u16 fragment index.
outputs:
    0 when the fragment was encoded, 1 when the encoding failed.
ownership:
    The output is an exclusive ins loan that is written; the payload is borrowed.
errors:
    A WireError from wire_fragment_encode maps to 1.
side_effects:
    Writes the fragment into the output buffer.
"""
verb make_fragment(ins output: Buffer, abs payload: Buffer, erg offset: u32, erg index: u16) -> Int {
    erg header = WireFragmentHeader {storage: Array[u8, 17]()};
    header.version = WIRE_FRAGMENT_VERSION;
    header.fragment_index = index;
    header.fragment_count = 2u16;
    header.total_length = 6u32;
    header.offset = offset;
    header.generation = 9u32;
    erg encoded = wire_fragment_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat encoded { Result.Ok(_) => 0, Result.Err(_) => 1, };
}
"""
contract:
purpose:
    Accept the first fragment of a two-fragment message into a reassembly and check that the same fragment is then rejected as a duplicate.
inputs:
    reassembly: exclusive ins loan of WireReassembly in progress.
outputs:
    0 on success, 2 when the fragment cannot be built, 3 when the first accept fails or completes, 4 when the duplicate is not reported.
ownership:
    The reassembly is an exclusive ins loan that is updated on acceptance.
errors:
    The duplicate fragment error is the expected outcome of the second accept.
side_effects:
    Updates the reassembly state.
"""
verb accept_first(ins reassembly: WireReassembly) -> Int {
    erg first_payload = Buffer[3];
    first_payload[0] = 65u8;
    first_payload[1] = 66u8;
    first_payload[2] = 67u8;
    erg first = Buffer[20];
    erg first_offset: u32 = 0u32;
    erg first_index: u16 = 0u16;
    if make_fragment(output: ins first, payload: abs first_payload, offset: erg first_offset, index: erg first_index) != 0 { return 2; }
    erg accepted = wire_reassembly_accept(reassembly: ins reassembly, fragment: abs first);
    if case dat accepted { Result.Ok(value) => value != 0, Result.Err(_) => true, } { return 3; }
    erg duplicate = wire_reassembly_accept(reassembly: ins reassembly, fragment: abs first);
    if case dat duplicate { Result.Err(error) => case dat error { WireError.FragmentDuplicate => false, _ => true, }, Result.Ok(_) => true, } { return 4; }
    return 0;
}
"""
contract:
purpose:
    Accept the second fragment, check that the message completes, and copy the six reassembled bytes ABCDEF to an output buffer.
inputs:
    reassembly: exclusive ins loan of WireReassembly with the first fragment accepted.
outputs:
    0 on success, 5 when the fragment cannot be built, 6 when the accept fails, 7 when the copied bytes differ, 8 when the copy fails.
ownership:
    The reassembly is an exclusive ins loan that is updated and then read.
errors:
    A WireError from accept or copy maps to 6 or 8.
side_effects:
    Updates the reassembly state.
"""
verb complete_second(ins reassembly: WireReassembly) -> Int {
    erg second_payload = Buffer[3];
    second_payload[0] = 68u8;
    second_payload[1] = 69u8;
    second_payload[2] = 70u8;
    erg second = Buffer[20];
    erg second_offset: u32 = 3u32;
    erg second_index: u16 = 1u16;
    if make_fragment(output: ins second, payload: abs second_payload, offset: erg second_offset, index: erg second_index) != 0 { return 5; }
    erg completed = wire_reassembly_accept(reassembly: ins reassembly, fragment: abs second);
    if case dat completed { Result.Ok(_) => false, Result.Err(_) => true, } { return 6; }
    erg output = Buffer[6];
    erg copied = wire_reassembly_copy(reassembly: abs reassembly, output: ins output);
    return case dat copied { Result.Ok(value) => if value == 6u32 && output[0] == 65u8 && output[5] == 70u8 { 0 } else { 7 }, Result.Err(_) => 8, };
}
"""Entry point that opens a reassembly over six bytes, begins a two-fragment message, accepts the first fragment and completes it with the second, returning 0 or the code of the first failing step."""
verb main() -> Int {
    erg storage = Buffer[6];
    erg reassembly = wire_reassembly_open(storage: dat storage);
    erg total_length: u32 = 6u32;
    erg fragment_count: u16 = 2u16;
    erg generation: u32 = 9u32;
    erg started = wire_reassembly_begin(reassembly: ins reassembly, total_length: erg total_length, fragment_count: erg fragment_count, generation: erg generation);
    if case dat started { Result.Ok(_) => false, Result.Err(_) => true, } { return 1; }
    if accept_first(reassembly: ins reassembly) != 0 { return 2; }
    return complete_second(reassembly: ins reassembly);
}
"#,
    )
    .expect("source");
    let output = root.join("wire-fragment");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let execution = Command::new(&output).output().expect("execute");
    assert_eq!(
        execution.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&execution.stdout),
        String::from_utf8_lossy(&execution.stderr)
    );
    let _ = fs::remove_dir_all(root);
}

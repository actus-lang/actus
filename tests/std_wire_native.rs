use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_actus"))
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
        r#"import std::wire;
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
verb invalid_magic_status() -> Int {
    erg input: Buffer = Buffer[12];
    input[0] = 0u8;
    input[1] = WIRE_MAGIC_1;
    input[2] = WIRE_VERSION;
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.InvalidMagic => 0, _ => 1 }, _ => 1 };
}
verb invalid_version_status() -> Int {
    erg input: Buffer = Buffer[12];
    input[0] = WIRE_MAGIC_0;
    input[1] = WIRE_MAGIC_1;
    input[2] = 2u8;
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.UnsupportedVersion => 0, _ => 1 }, _ => 1 };
}
verb unknown_flags_status() -> Int {
    erg input: Buffer = Buffer[12];
    input[0] = WIRE_MAGIC_0;
    input[1] = WIRE_MAGIC_1;
    input[2] = WIRE_VERSION;
    input[3] = 1u8;
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.UnknownFlags => 0, _ => 1 }, _ => 1 };
}
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
verb truncated_header_status() -> Int {
    erg input: Buffer = Buffer[11];
    erg decoded = wire_header_decode(input: abs input);
    return case dat decoded { Result.Err(error) => case dat error { WireError.Truncated => 0, _ => 1 }, _ => 1 };
}
verb frame_count_status(abs header: WireHeader, abs payload: Buffer, abs expected_count: u32) -> Int {
    erg output = Buffer[1038];
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat result {
        Result.Ok(count) => if count == expected_count { 0 } else { 1 },
        Result.Err(_) => 2,
    };
}
verb oversized_status(abs header: WireHeader) -> Int {
    erg payload: Buffer = Buffer[1025];
    erg output = Buffer[1039];
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat result {
        Result.Err(error) => case dat error { WireError.PayloadTooLarge => 0, _ => 1 },
        Result.Ok(_) => 2,
    };
}
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
verb deterministic_encode_status(abs header: WireHeader, abs payload: Buffer, ins output: Buffer) -> Int {
    erg result = wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
    return case dat result {
        Result.Ok(_) => 0,
        Result.Err(_) => 1,
    };
}
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
        r#"import std::wire;
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
        r#"import std::wire;
verb accept_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Ok(_) => 0, Result.Err(_) => 1, };
}

verb duplicate_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceDuplicate => 0, _ => 1, }, Result.Ok(_) => 2, };
}
verb stale_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceStale => 0, _ => 1, }, Result.Ok(_) => 2, };
}
verb jump_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceJumpTooLarge => 0, _ => 1, }, Result.Ok(_) => 2, };
}
verb context_status(ins window: WireSequenceWindow, erg context_id: u32, erg sequence_num: u32) -> Int {
    erg result = wire_sequence_accept(window: ins window, context_id: erg context_id, sequence_num: erg sequence_num);
    return case dat result { Result.Err(error) => case dat error { WireError.SequenceContextMismatch => 0, _ => 1, }, Result.Ok(_) => 2, };
}
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
        r#"import std::wire;
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

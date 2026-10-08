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

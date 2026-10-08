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
    erg decoded = wire_frame_decode(input: abs encoded, payload_output: ins payload);
    return case dat decoded {
        Result.Ok(_) => 2,
        Result.Err(error) => case dat error {
            WireError.ChecksumMismatch => 0,
            _ => 1,
        },
    };
}
verb main() -> Int {
    if crc_status() != 0 {
        return 7;
    }
    return header_status();
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
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

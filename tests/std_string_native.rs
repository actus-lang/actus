use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_actus"))
}

#[test]
fn hosted_string_byte_access_handles_ascii_multibyte_empty_and_bounds() {
    let root = std::env::temp_dir().join(format!("actus-string-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"string_access\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    fs::write(
        root.join("src/main.act"),
        r#"meta limitless("file")
import std::string;
verb inspect_owned(dat value: Utf8Buffer) -> Int {
    erg length_result = utf8_length(text: abs value);
    drop(value);
    return case dat length_result {
        Result.Ok(length) => if length == 3 { 0 } else { 1 },
        Result.Err(_) => 3,
    };
}
verb owned_utf8_code() -> Int {
    erg storage = Buffer[0];
    append(storage, 65);
    erg source: String = "AB";
    erg result = utf8_from_string(text: abs source, storage: dat storage);
    return case dat result {
        Result.Ok(value) => inspect_owned(value: dat value),
        Result.Err(_) => 2,
    };
}
verb embedded_nul_code() -> Int {
    erg storage = Buffer[0];
    append(storage, 65);
    append(storage, 0);
    append(storage, 66);
    erg result = utf8_from_buffer(storage: dat storage);
    return case dat result {
        Result.Ok(value) => {
            erg length_result = utf8_length(text: abs value);
            return case dat length_result {
            Result.Ok(length) => if length == 3 { 0 } else { 1 },
            Result.Err(_) => 3,
            };
        },
        Result.Err(_) => 2,
    };
}
verb bounds_ok(erg result: Result[Int, StringError]) -> Int {
    return case dat result {
        Result.Err(error) => case dat error {
            StringError.OutOfBounds => 0,
            _ => 1,
        },
        Result.Ok(_) => 2,
    };
}
verb main() -> Int {
    if owned_utf8_code() != 0 {
        return 7;
    }
    if embedded_nul_code() != 0 {
        return 8;
    }
    erg ascii_text: String = "Actus";
    erg utf_text: String = "é";
    erg empty_text: String = "";
    erg one: Int = 1;
    erg two: Int = 2;
    erg ascii = string_length(text: abs ascii_text);
    erg utf = string_length(text: abs utf_text);
    erg empty = string_length(text: abs empty_text);
    erg byte = string_byte_at(text: abs utf_text, index: erg one);
    erg bounds = string_byte_at(text: abs utf_text, index: erg two);
    erg bounds_code = bounds_ok(result: erg bounds);
    return case ascii {
        Result.Ok(value) => case utf {
            Result.Ok(utf_len) => case empty {
                Result.Ok(empty_len) => case byte {
                    Result.Ok(last) => if value == 5 && utf_len == 2 && empty_len == 0 && last == 169 && bounds_code == 0 { 0 } else { 1 },
                    _ => 3,
                },
                _ => 4,
            },
            _ => 5,
        },
        _ => 6,
    };
}
"#,
    )
    .expect("source");
    let output = root.join("string-access");
    let object = root.join("string-access.o");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let object_build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .output()
        .expect("object build");
    assert!(object_build.status.success(), "{}", String::from_utf8_lossy(&object_build.stderr));
    let run = Command::new(&output).output().expect("run");
    assert_eq!(run.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_io_and_string_facades_share_compatible_buffer_bridge() {
    let root = std::env::temp_dir().join(format!("actus-io-string-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"io_string_bridge\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    fs::write(
        root.join("src/main.act"),
        r#"import std::io;
import std::string;

verb main() -> Int {
    erg text: String = "abc";
    erg text_result = string_length(text: abs text);
    erg buffer = Buffer[0];
    append(buffer, 65);
    erg buffer_result = buffer_size(buffer: abs buffer);
    return case text_result {
        Result.Ok(text_length) => case buffer_result {
            Result.Ok(buffer_length_value) => if text_length == 3 && buffer_length_value == 1 { 0 } else { 1 },
            Result.Err(_) => 2,
        },
        Result.Err(_) => 3,
    };
}
"#,
    )
    .expect("source");
    let input = root.join("src/main.act");
    let object = root.join("io-string-bridge.o");
    let output = root.join("io-string-bridge");
    let check = Command::new(compiler())
        .current_dir(&root)
        .args(["check", "--strict"])
        .output()
        .expect("check");
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));
    let object_build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .output()
        .expect("object build");
    assert!(object_build.status.success(), "{}", String::from_utf8_lossy(&object_build.stderr));
    let executable_build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("executable build");
    assert!(
        executable_build.status.success(),
        "{}",
        String::from_utf8_lossy(&executable_build.stderr)
    );
    let run = Command::new(&output).current_dir(&root).output().expect("run");
    assert_eq!(run.status.code(), Some(0));
    assert!(input.is_file());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_nested_facade_preserves_borrowed_string_arguments() {
    let root = std::env::temp_dir().join(format!("actus-nested-string-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/text/tokenizer")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"nested_string\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("manifest");
    fs::write(root.join("src/text/text.act"), "open tokenizer;\n").expect("facade");
    fs::write(root.join("src/text/tokenizer/tokenizer.act"), "open byte;\n")
        .expect("nested facade");
    fs::write(
        root.join("src/text/tokenizer/byte.act"),
        r#"import std::string;
open const TOKEN_VERSION: u16 = 1u16;
open struct ByteRecord { id: u32, offset: u32, length: u16, version: u16, }
open struct TokenSequence { records: Array[ByteRecord, 256], length: u32, valid: Bool, status: Int, }
open verb inspect(abs text: String) -> TokenSequence {
    erg sequence: TokenSequence = TokenSequence { records: Array[ByteRecord, 256](), length: 0u32, valid: true, status: 1 };
    erg length_result = string_length(text: abs text);
    erg text_length: u32 = case dat length_result {
        Result.Ok(value) => value as u32,
        Result.Err(_) => 0u32,
    };
    erg position: u32 = 0u32;
    loop {
        if position >= text_length { break; }
        erg byte_index: Int = position as Int;
        erg result = string_byte_at(text: abs text, index: erg byte_index);
        erg token: ByteRecord = ByteRecord { id: 0u32, offset: 0u32, length: 0u16, version: TOKEN_VERSION };
        case dat result {
            Result.Ok(byte) => {
                token = ByteRecord { id: byte as u32, offset: position, length: 1u16, version: TOKEN_VERSION };
                sequence.records[position as Usize] = token;
            },
            Result.Err(_) => {},
        };
        if token.length == 0u16 {
            sequence.status = 2;
            sequence.valid = false;
            return sequence;
        }
        sequence.length = position + 1u32;
        position += 1u32;
    }
    if sequence.valid && sequence.length == text_length
        && sequence.records[0u32].id == 72u32
        && sequence.records[11u32].id == 105u32
    {
        sequence.status = 0;
    }
    return sequence;
}
"#,
    )
    .expect("nested implementation");
    fs::write(
        root.join("src/main.act"),
        r#"import text;
import std::io;
import std::time;
import std::string;
meta limitless("verb")
verb main() -> Int {
    erg phrase: String = "Hello Giorgi";
    erg direct_index: Int = 0;
    erg direct_result = string_byte_at(text: abs phrase, index: erg direct_index);
    erg result = inspect(text: abs phrase);
    return result.status;
}
"#,
    )
    .expect("source");
    let object = root.join("nested-string.o");
    let output = root.join("nested-string");
    let check = Command::new(compiler())
        .current_dir(&root)
        .args(["check", "--strict"])
        .output()
        .expect("check");
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));
    let object_build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .output()
        .expect("object build");
    assert!(object_build.status.success(), "{}", String::from_utf8_lossy(&object_build.stderr));
    assert!(
        String::from_utf8_lossy(&object_build.stdout)
            .contains("verified generated native IR: no floating-point instructions")
    );
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .output()
        .expect("build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let run = Command::new(&output).current_dir(&root).output().expect("run");
    assert_eq!(run.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

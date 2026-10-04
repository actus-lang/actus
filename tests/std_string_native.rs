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
    erg buffer_result = buffer_length(buffer: abs buffer);
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

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
verb main() -> Int {
    erg input: Buffer = Buffer[0];
    append(input, 1u8);
    append(input, 2u8);
    append(input, 3u8);
    erg start: u32 = 0u32;
    erg end: u32 = 3u32;
    erg result = wire_crc16_ccitt(input: abs input, start: erg start, end: erg end);
    return case dat result {
        Result.Ok(value) => if value == 44461u16 { 0 } else { 1 },
        Result.Err(_) => 2,
    };
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

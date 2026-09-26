use std::fs;
use std::path::{Path, PathBuf};

use actus::cli::run_with_args;
use actus::runtime::{
    actus_file_close, actus_file_flush, actus_file_open, actus_file_read, actus_file_write,
};

fn copy_library(root: &Path, files: &[(&str, String)]) {
    for (relative, source) in files {
        let destination = root.join("src").join(relative);
        fs::create_dir_all(destination.parent().expect("library parent"))
            .expect("create library directory");
        fs::write(destination, source).expect("write library source");
    }
}

fn library_file(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src").join(relative);
    fs::read_to_string(path).expect("standard library source should exist")
}

fn path_literal(path: &Path) -> String {
    path.to_string_lossy()
        .bytes()
        .map(|byte| format!("append(path, {});", byte))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn std_fs_api_builds_with_reader_writer_and_drop_contracts() {
    let root = std::env::temp_dir().join(format!("actus-fs-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create fixture root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"fs-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write fixture manifest");
    let relative_files = [
        ("lib.act", library_file("lib.act")),
        ("io/io.act", library_file("io/io.act")),
        ("io/error.act", library_file("io/error.act")),
        ("io/reader.act", library_file("io/reader.act")),
        ("io/writer.act", library_file("io/writer.act")),
        ("io/stdout.act", library_file("io/stdout.act")),
        ("io/stderr.act", library_file("io/stderr.act")),
        ("io/stdin.act", library_file("io/stdin.act")),
        ("io/read.act", library_file("io/read.act")),
        ("io/write.act", library_file("io/write.act")),
        ("io/buffered.act", library_file("io/buffered.act")),
        ("io/cursor.act", library_file("io/cursor.act")),
        ("io/copy.act", library_file("io/copy.act")),
        ("fs/fs.act", library_file("fs/fs.act")),
        ("fs/file.act", library_file("fs/file.act")),
    ];
    copy_library(&root, &relative_files);
    let data_path = root.join("roundtrip.bin");
    let path = path_literal(&data_path);
    let source = format!(
        "import io; import fs; verb round_trip() -> Result[Int, IoError] {{ erg path = Buffer[0]; {path} erg payload = Buffer[0]; append(payload, 65); append(payload, 66); erg created_result = file_create(path: abs path); case dat created_result {{ Result.Ok(created) => {{ erg file: File = created; file.write(buffer: abs payload); file.flush(); }}, Result.Err(_) => {{ return Result[Int, IoError].Ok(0); }}, }}; erg reopened_result = file_open(path: abs path); case dat reopened_result {{ Result.Ok(reopened) => {{ erg file: File = reopened; erg destination = Buffer[128]; erg read_result = file.read(buffer: ins destination); return case dat read_result {{ Result.Ok(count) => Result[Int, IoError].Ok(count), Result.Err(_) => Result[Int, IoError].Ok(0), }}; }}, Result.Err(_) => {{ return Result[Int, IoError].Ok(0); }}, }}; return Result[Int, IoError].Ok(0); }} verb main() -> Int {{ round_trip(); return 0; }}"
    );
    let input = source_root.join("main.act");
    fs::write(&input, source).expect("write Actus fixture");
    let output = root.join("fixture");
    let status = run_with_args(
        [
            "build",
            input.to_str().expect("fixture path"),
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path"),
        ]
        .into_iter()
        .map(str::to_owned),
    );
    assert_eq!(status, 0);
    let execution = std::process::Command::new(&output).status().expect("run fs fixture");
    assert_eq!(execution.code(), Some(0));
    assert_eq!(fs::read(&data_path).expect("read round-trip file"), b"AB");
    fs::remove_dir_all(root).expect("remove fixture root");
}

#[test]
fn runtime_file_bridge_round_trips_and_closes_handles() {
    let path = std::env::temp_dir().join(format!("actus-runtime-fs-{}.bin", std::process::id()));
    let path_bytes = path.to_string_lossy().into_owned().into_bytes();
    let handle = unsafe { actus_file_open(path_bytes.as_ptr(), path_bytes.len(), 1) };
    assert!(handle >= 0);
    let payload = b"AB";
    let written = unsafe { actus_file_write(handle, payload.as_ptr(), payload.len()) };
    assert_eq!(written, 2);
    assert_eq!(actus_file_flush(handle), 0);
    assert_eq!(actus_file_close(handle), 0);

    let reopened = unsafe { actus_file_open(path_bytes.as_ptr(), path_bytes.len(), 0) };
    assert!(reopened >= 0);
    let mut destination = [0_u8; 2];
    let read = unsafe { actus_file_read(reopened, destination.as_mut_ptr(), destination.len()) };
    assert_eq!(read, 2);
    assert_eq!(&destination, payload);
    assert_eq!(actus_file_close(reopened), 0);
    fs::remove_file(path).expect("remove runtime fixture");
}

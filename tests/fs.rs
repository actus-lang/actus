use std::fs;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
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

#[cfg(unix)]
fn path_literal(path: &Path) -> String {
    let mut source = path
        .to_string_lossy()
        .bytes()
        .map(|byte| format!("append(raw, {});", byte))
        .collect::<Vec<_>>()
        .join(" ");
    source.push_str(" append(raw, 0);");
    source.push_str(" erg path = make_path(raw: dat raw);");
    source
}

#[cfg(windows)]
fn path_literal(path: &Path) -> String {
    let mut source = path
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_ne_bytes)
        .map(|byte| format!("append(raw, {});", byte))
        .collect::<Vec<_>>()
        .join(" ");
    source.push_str(" append(raw, 0); append(raw, 0);");
    source.push_str(" erg path = make_path(raw: dat raw);");
    source
}

#[cfg(unix)]
fn path_constructor() -> &'static str {
    "path_from_posix"
}

#[cfg(windows)]
fn path_constructor() -> &'static str {
    "path_from_windows_utf16"
}

#[cfg(unix)]
fn fallback_path_fields() -> (&'static str, &'static str, &'static str) {
    ("Buffer[1]", "PathPlatform.Posix", "1")
}

#[cfg(windows)]
fn fallback_path_fields() -> (&'static str, &'static str, &'static str) {
    ("Buffer[2]", "PathPlatform.Windows", "2")
}

fn make_path_source() -> String {
    let (storage, platform, capacity) = fallback_path_fields();
    format!(
        "verb make_path(dat raw: Buffer) -> Path {{ erg result = {}(storage: dat raw); return case dat result {{ Result.Ok(value) => value, Result.Err(_) => Path {{ storage: {storage}, platform: {platform}, length: 0, capacity: {capacity}, terminated: 1, }}, }}; }}",
        path_constructor(),
    )
}

fn path_literal_for(path: &Path, binding: &str) -> String {
    let raw_binding = format!("{binding}_raw");
    path_literal(path).replace("raw,", &format!("{raw_binding},")).replace(
        "erg path = make_path(raw: dat raw);",
        &format!("erg {binding} = make_path(raw: dat {raw_binding});"),
    )
}

fn prepare_source(source: &str) -> String {
    let source = source
        .replace("erg path = Buffer[0];", "erg raw = Buffer[0];")
        .replace("erg renamed = Buffer[0];", "erg renamed_raw = Buffer[0];")
        .replace("erg source = Buffer[0];", "erg source_raw = Buffer[0];")
        .replace("import io; import fs;", "import io; import path; import fs;");
    format!("{source}\n{}", make_path_source())
}

fn copy_std_library(root: &Path) {
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
        ("fs/metadata.act", library_file("fs/metadata.act")),
        ("fs/operations.act", library_file("fs/operations.act")),
        ("fs/seek.act", library_file("fs/seek.act")),
        ("fs/options.act", library_file("fs/options.act")),
        ("path/path.act", library_file("path/path.act")),
        ("path/types.act", library_file("path/types.act")),
        ("path/storage.act", library_file("path/storage.act")),
        ("path/error.act", library_file("path/error.act")),
        ("path/components.act", library_file("path/components.act")),
        ("path/posix.act", library_file("path/posix.act")),
        ("path/windows.act", library_file("path/windows.act")),
        ("path/predicates.act", library_file("path/predicates.act")),
        ("path/normalize.act", library_file("path/normalize.act")),
        ("path/builders.act", library_file("path/builders.act")),
    ];
    copy_library(root, &relative_files);
}

fn write_fixture(root: &Path, name: &str, source: &str) -> (PathBuf, PathBuf) {
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create fixture root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"fs-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write fixture manifest");
    copy_std_library(root);
    let input = source_root.join(name);
    fs::write(&input, prepare_source(source)).expect("write Actus fixture");
    (input, root.join("fixture"))
}

fn build_and_run(input: &Path, output: &Path) {
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
    let execution = std::process::Command::new(output).status().expect("run fs fixture");
    assert_eq!(execution.code(), Some(0));
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
    copy_std_library(&root);
    let data_path = root.join("roundtrip.bin");
    let path = path_literal(&data_path);
    let source = format!(
        "import io; import fs; verb round_trip() -> Result[Int, IoError] {{ erg path = Buffer[0]; {path} erg payload = Buffer[0]; append(payload, 65); append(payload, 66); erg created_result = file_create(path: abs path); case dat created_result {{ Result.Ok(created) => {{ erg file: File = created; file.write(buffer: abs payload); file.flush(); }}, Result.Err(_) => {{ return Result[Int, IoError].Ok(0); }}, }}; erg reopened_result = file_open(path: abs path); case dat reopened_result {{ Result.Ok(reopened) => {{ erg file: File = reopened; erg destination = Buffer[128]; erg read_result = file.read(buffer: ins destination); return case dat read_result {{ Result.Ok(count) => Result[Int, IoError].Ok(count), Result.Err(_) => Result[Int, IoError].Ok(0), }}; }}, Result.Err(_) => {{ return Result[Int, IoError].Ok(0); }}, }}; return Result[Int, IoError].Ok(0); }} verb main() -> Int {{ round_trip(); return 0; }}"
    );
    let input = source_root.join("main.act");
    fs::write(&input, prepare_source(&source)).expect("write Actus fixture");
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
fn std_fs_seek_reads_from_start_after_writing() {
    let root = std::env::temp_dir().join(format!("actus-fs-seek-{}", std::process::id()));
    let data_path = root.join("seek.bin");
    let path = path_literal(&data_path);
    let source = format!(
        "import io; import fs; verb seek_round_trip() -> Result[Int, IoError] {{ erg path = Buffer[0]; {path} erg payload = Buffer[0]; append(payload, 65); append(payload, 66); erg created = file_create(path: abs path); case dat created {{ Result.Ok(value) => {{ erg file: File = value; erg written = file.write(buffer: abs payload); case dat written {{ Result.Ok(_) => 0, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; erg origin = SeekFrom.Start(0); erg moved = file.seek(from: dat origin); case dat moved {{ Result.Ok(_) => 0, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; erg destination = Buffer[0]; erg capacity = 2; reserve(buffer: ins destination, capacity: capacity); erg result = file.read(buffer: ins destination); case dat result {{ Result.Ok(count) => {{ return Result[Int, IoError].Ok(count); }}, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; }}, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; return Result[Int, IoError].Err(IoError.Failed); }} verb main() -> Int {{ erg result = seek_round_trip(); return case dat result {{ Result.Ok(count) => case count {{ 2 => 0, _ => 1, }}, Result.Err(_) => 99, }}; }}"
    );
    let (input, output) = write_fixture(&root, "main.act", &source);
    build_and_run(&input, &output);
    assert_eq!(fs::read(&data_path).expect("read seek fixture"), b"AB");
    fs::remove_dir_all(root).expect("remove seek fixture");
}

#[test]
fn std_fs_open_options_append_preserves_existing_bytes() {
    let root = std::env::temp_dir().join(format!("actus-fs-append-{}", std::process::id()));
    let data_path = root.join("append.bin");
    let path = path_literal(&data_path);
    let source = format!(
        "import io; import fs; verb append_round_trip() -> Result[Int, IoError] {{ erg path = Buffer[0]; {path} erg first = Buffer[0]; append(first, 65); erg created = file_create(path: abs path); case dat created {{ Result.Ok(value) => {{ erg file: File = value; erg written = file.write(buffer: abs first); case dat written {{ Result.Ok(_) => 0, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; }}, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; erg empty = options_new(); erg writable = options_write(dat empty); erg appendable = options_append(dat writable); erg opened = options_open(options: dat appendable, path: abs path); case dat opened {{ Result.Ok(value) => {{ erg file: File = value; erg second = Buffer[0]; append(second, 66); erg written = file.write(buffer: abs second); case dat written {{ Result.Ok(_) => 0, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; }}, Result.Err(error) => {{ return Result[Int, IoError].Err(error); }}, }}; return Ok(2); }} verb main() -> Int {{ erg result = append_round_trip(); return case dat result {{ Result.Ok(count) => case count {{ 2 => 0, _ => 1, }}, Result.Err(_) => 1, }}; }}"
    );
    let (input, output) = write_fixture(&root, "main.act", &source);
    build_and_run(&input, &output);
    assert_eq!(fs::read(&data_path).expect("read append fixture"), b"AB");
    fs::remove_dir_all(root).expect("remove append fixture");
}

#[test]
fn std_fs_metadata_reports_file_size_and_kind() {
    let root = std::env::temp_dir().join(format!("actus-fs-metadata-{}", std::process::id()));
    let data_path = root.join("metadata.bin");
    fs::create_dir_all(&root).expect("create metadata fixture");
    fs::write(&data_path, b"AB").expect("write metadata fixture");
    let path = path_literal(&data_path);
    let source = format!(
        "import io; import fs; verb main() -> Int {{ erg path = Buffer[0]; {path} erg result = metadata(path: abs path); return case dat result {{ Result.Ok(value) => {{ erg info: Metadata = value; return case info.size {{ 2 => case info.is_file {{ 1 => case info.is_dir {{ 0 => 0, _ => 1, }}, _ => 1, }}, _ => 1, }}; }}, Result.Err(_) => 2, }}; }}"
    );
    let (input, output) = write_fixture(&root, "main.act", &source);
    build_and_run(&input, &output);
    let metadata = fs::metadata(&data_path).expect("read metadata fixture");
    assert_eq!(metadata.len(), 2);
    assert!(metadata.is_file());
    fs::remove_dir_all(root).expect("remove metadata fixture");
}

#[test]
fn std_fs_rename_and_remove_file_execute_natively() {
    let root = std::env::temp_dir().join(format!("actus-fs-rename-{}", std::process::id()));
    let source_path = root.join("before.bin");
    let renamed_path = root.join("after.bin");
    let source_literal = path_literal_for(&source_path, "source");
    let renamed_literal = path_literal_for(&renamed_path, "renamed");
    fs::create_dir_all(&root).expect("create rename fixture");
    fs::write(&source_path, b"A").expect("write rename source");
    let rename_source = format!(
        "import io; import fs; verb main() -> Int {{ erg renamed = Buffer[0]; {renamed_literal} erg source = Buffer[0]; {source_literal} erg moved = rename(from: abs source, to: abs renamed); return case dat moved {{ Result.Ok(_) => 0, Result.Err(_) => 1, }}; }}"
    );
    let (input, output) = write_fixture(&root, "main.act", &rename_source);
    build_and_run(&input, &output);
    assert!(!source_path.exists());
    assert!(renamed_path.exists());

    let remove_source = format!(
        "import io; import fs; verb main() -> Int {{ erg renamed = Buffer[0]; {renamed_literal} erg removed = remove_file(path: abs renamed); return case dat removed {{ Result.Ok(_) => 0, Result.Err(_) => 1, }}; }}"
    );
    fs::write(&input, prepare_source(&remove_source)).expect("write remove fixture");
    build_and_run(&input, &output);
    assert!(!renamed_path.exists());
    fs::remove_dir_all(root).expect("remove rename fixture");
}

#[test]
fn std_fs_one_shot_write_and_read_to_string_execute_natively() {
    let root = std::env::temp_dir().join(format!("actus-fs-one-shot-{}", std::process::id()));
    let data_path = root.join("one-shot.bin");
    let path = path_literal(&data_path);
    let source = format!(
        "import io; import fs; verb main() -> Int {{ erg path = Buffer[0]; {path} erg contents = Buffer[0]; append(contents, 88); append(contents, 89); erg written = write_file(path: abs path, contents: abs contents); return case dat written {{ Result.Ok(_) => {{ erg loaded = read_to_string(path: abs path); return case dat loaded {{ Result.Ok(_) => 0, Result.Err(_) => 2, }}; }}, Result.Err(_) => 1, }}; }}"
    );
    let (input, output) = write_fixture(&root, "main.act", &source);
    build_and_run(&input, &output);
    assert_eq!(fs::read(&data_path).expect("read one-shot fixture"), b"XY");
    fs::remove_dir_all(root).expect("remove one-shot fixture");
}

#[test]
fn std_fs_read_to_bytes_and_rejects_invalid_utf8() {
    let root = std::env::temp_dir().join(format!("actus-fs-bytes-{}", std::process::id()));
    let data_path = root.join("bytes.bin");
    let path = path_literal(&data_path);
    let source = format!(
        "import io; import fs; verb main() -> Int {{ erg path = Buffer[0]; {path} erg contents = Buffer[0]; append(contents, 255); write_file(path: abs path, contents: abs contents); erg bytes = read_to_bytes(path: abs path); case dat bytes {{ Result.Ok(_) => {{ erg text = read_to_string(path: abs path); return case dat text {{ Result.Err(_) => 0, Result.Ok(_) => 1, }}; }}, Result.Err(_) => {{ return 1; }}, }}; return 1; }}"
    );
    let (input, output) = write_fixture(&root, "main.act", &source);
    build_and_run(&input, &output);
    assert_eq!(fs::read(&data_path).expect("read invalid UTF-8 fixture"), [255]);
    fs::remove_dir_all(root).expect("remove bytes fixture");
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

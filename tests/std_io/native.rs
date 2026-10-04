use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use actus::cli::run_with_args;
use object::{Object, ObjectSymbol};

use super::fixtures::library_source;

fn project(name: &str, facade: &str, files: &[&str], source: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("actus-std-{name}-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create std io fixture");
    fs::write(io_root.join("io.act"), facade).expect("write io facade");
    for file in files {
        fs::write(io_root.join(file), library_source(file)).expect("copy std io source");
    }
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"std-{name}-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n"
        ),
    )
    .expect("write fixture manifest");
    let input = source_root.join("main.act");
    fs::write(&input, source).expect("write fixture entry");
    let output = root.join(name);
    (root, input, output)
}

fn build(input: &Path, output: &Path) {
    assert_eq!(
        run_with_args(
            vec![
                "build".to_owned(),
                input.display().to_string(),
                "--emit".to_owned(),
                "exe".to_owned(),
                "-o".to_owned(),
                output.display().to_string(),
            ]
            .into_iter(),
        ),
        0
    );
}

#[test]
fn std_io_native_buffer_flushes_pending_bytes() {
    let (root, input, output) = project(
        "buffered",
        "open stdout;\nopen error;\n",
        &["stdout.act", "error.act"],
        "import io; verb main() -> Int { erg storage = Buffer[0]; append(storage, 97); append(storage, 98); append(storage, 99); append(storage, 100); append(storage, 101); append(storage, 102); printb(text: abs storage); flush(); return 0; }\n",
    );
    build(&input, &output);
    let child =
        Command::new(&output).stdout(Stdio::piped()).spawn().expect("buffered fixture should run");
    let execution = child.wait_with_output().expect("wait for buffered fixture");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"abcdef");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_bridge_separates_stdout_and_stderr() {
    let (root, input, output) = project(
        "streams",
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n",
        &["stdout.act", "stderr.act", "stdin.act", "error.act"],
        "import io; verb main() -> Int { erg value = 42; print_int(value: value); eprint_int(value: value); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("std io fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"42\n");
    assert_eq!(execution.stderr, b"42\n");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_bridge_prints_borrowed_buffer_text() {
    let (root, input, output) = project(
        "text",
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n",
        &["stdout.act", "stderr.act", "stdin.act", "error.act"],
        "import io; verb main() -> Int { erg text = Buffer[0]; append(text, 72); append(text, 105); printb(text: abs text); printlnb(text: abs text); eprint(text: abs text); eprintln(text: abs text); flush(); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("text fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"HiHi\n");
    assert_eq!(execution.stderr, b"HiHi\n");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_bridge_prints_string_text() {
    let (root, input, output) = project(
        "string-text",
        "open stdout;\nopen error;\n",
        &["stdout.act", "error.act"],
        "import io; verb main() -> Int { erg text = \"Hello\"; print(abs text); println(text: abs text); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("string fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"HelloHello\n");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_string_collection_covers_case_blocks_and_else_if() {
    let (root, input, output) = project(
        "string-control-flow",
        "open stdout;\nopen error;\n",
        &["stdout.act", "error.act"],
        r#"import io;
verb main() -> Int {
    erg selected = "selected";
    case true {
        true if false => {
            erg guard_text = "guard branch";
            print(abs guard_text);
        },
        _ => {
            print(abs selected);
        },
    };
    if false {
        erg unreachable = "unreachable";
        print(abs unreachable);
    } else if true {
        erg nested = "else-if branch";
        println(abs nested);
    }
    return 0;
}
"#,
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("string control-flow fixture");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"selectedelse-if branch\n");
    assert_eq!(execution.stderr, b"");
    let object = output.with_extension("obj");
    let object_build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build string control-flow object");
    assert!(object_build.status.success(), "object build failed: {:?}", object_build);
    assert!(object.is_file());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_string_data_is_deduplicated_and_deterministic() {
    let (root, input, output) = project(
        "string-data",
        "open stdout;\nopen error;\n",
        &["stdout.act", "error.act"],
        r#"import io;
verb main() -> Int {
    erg first = "zeta";
    erg repeated = "alpha";
    print(abs first);
    println(abs repeated);
    return 0;
}
"#,
    );
    let first = output.with_extension("first.obj");
    let second = output.with_extension("second.obj");
    for object in [&first, &second] {
        let result = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args([
                "build",
                input.to_str().expect("source path"),
                "--strict",
                "--emit",
                "obj",
                "-o",
            ])
            .arg(object)
            .current_dir(&root)
            .output()
            .expect("build string object");
        assert!(result.status.success(), "object build failed: {:?}", result);
    }
    assert_eq!(fs::read(&first).expect("first object"), fs::read(&second).expect("second object"));
    assert_eq!(
        fs::read_to_string(root.join("string-data.first.symbols")).expect("first symbols"),
        fs::read_to_string(root.join("string-data.second.symbols")).expect("second symbols")
    );
    let bytes = fs::read(&first).expect("read string object");
    let object_file = object::File::parse(bytes.as_slice()).expect("parse string object");
    let string_symbols = object_file
        .symbols()
        .filter_map(|symbol| symbol.name().ok())
        .filter(|name| {
            name.contains("__data_string_")
                && !name.starts_with(".refptr.")
                && !name.starts_with("__imp_")
        })
        .collect::<Vec<_>>();
    assert_eq!(string_symbols.len(), 2, "string symbols: {string_symbols:?}");
    assert!(string_symbols.iter().any(|name| name.contains("__data_string_5f0")));
    assert!(string_symbols.iter().any(|name| name.contains("__data_string_5f1")));
    fs::remove_dir_all(root).expect("remove string data project");
}

#[test]
fn std_io_native_bridge_reads_line_into_exclusive_buffer() {
    let (root, input, output) = project(
        "stdin",
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n",
        &["stdout.act", "stderr.act", "stdin.act", "error.act"],
        "import io; verb main() -> Int { erg buffer = Buffer[0]; read_line(buffer: ins buffer); printb(text: abs buffer); flush(); return 0; }\n",
    );
    build(&input, &output);
    let mut child = Command::new(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("stdin fixture should run");
    child.stdin.take().expect("stdin pipe should exist").write_all(b"hello\n").unwrap();
    let execution = child.wait_with_output().expect("wait for stdin fixture");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"hello");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_bridge_reports_eof_from_read_byte() {
    let (root, input, output) = project(
        "byte",
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n",
        &["stdout.act", "stderr.act", "stdin.act", "error.act"],
        "import io; verb consume_byte() -> Result[Int, IoError] { read_byte()?; return Result[Int, IoError].Ok(0); } verb main() -> Int { consume_byte(); return 42; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("byte fixture should run");
    assert_eq!(execution.status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_stream_result_chain_propagates_success() {
    let (root, input, output) = project(
        "stream",
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\nopen read;\nopen write;\n",
        &["stdout.act", "stderr.act", "stdin.act", "error.act", "read.act", "write.act"],
        "import io; verb stream() -> Result[Int, IoError] { erg buffer = Buffer[0]; append(buffer, 115); append(buffer, 116); append(buffer, 114); append(buffer, 101); append(buffer, 97); append(buffer, 109); write(buffer: abs buffer)?; return Result[Int, IoError].Ok(0); } verb main() -> Int { stream(); return 42; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("stream fixture should run");
    assert_eq!(execution.status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_cursor_reader_uses_static_generic_dispatch() {
    let (root, input, output) = project(
        "cursor-reader",
        "open cursor;\nopen buffered;\nopen reader;\nopen writer;\nopen stdout;\nopen error;\n",
        &["cursor.act", "buffered.act", "reader.act", "writer.act", "stdout.act", "error.act"],
        "import io; verb main() -> Int { erg payload = Buffer[0]; append(payload, 67); append(payload, 117); append(payload, 114); erg storage = Buffer[0]; erg cursor = Cursor { buffer: storage, position: 0, }; cursor_write(self: ins cursor, buffer: abs payload); erg offset = 0; seek(self: ins cursor, offset: offset); erg output = Buffer[0]; cursor_read(self: ins cursor, buffer: ins output); printb(text: abs output); flush(); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("cursor fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"Cur");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_buffered_reader_specializes_cursor() {
    let (root, input, output) = project(
        "buffered-cursor",
        "open cursor;\nopen buffered;\nopen reader;\nopen writer;\nopen stdout;\nopen error;\n",
        &["cursor.act", "buffered.act", "reader.act", "writer.act", "stdout.act", "error.act"],
        "import io; verb main() -> Int { erg source_bytes = Buffer[0]; append(source_bytes, 72); append(source_bytes, 105); erg source = Cursor { buffer: source_bytes, position: 0, }; erg storage = Buffer[0]; erg reader: BufferedReader[Cursor] = BufferedReader[Cursor] { source: source, buffer: storage, }; refill(reader: ins reader); printb(text: abs reader.buffer); flush(); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("buffered Cursor fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"Hi");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_buffered_writer_specializes_cursor() {
    let (root, input, output) = project(
        "buffered-writer-cursor",
        "open cursor;\nopen buffered;\nopen reader;\nopen writer;\nopen stdout;\nopen error;\n",
        &["cursor.act", "buffered.act", "reader.act", "writer.act", "stdout.act", "error.act"],
        "import io; verb main() -> Int { erg storage = Buffer[0]; erg target = Cursor { buffer: storage, position: 0, }; erg scratch = Buffer[0]; erg capacity = 4; reserve(buffer: ins scratch, capacity: capacity); erg writer: BufferedWriter[Cursor] = BufferedWriter[Cursor] { target: target, buffer: scratch, }; erg payload = Buffer[0]; append(payload, 79); append(payload, 75); buffered_write(writer: ins writer, input: abs payload); flush_buffer(writer: ins writer); printb(text: abs writer.target.buffer); flush(); return 0; }\n",
    );
    build(&input, &output);
    let execution =
        Command::new(&output).output().expect("buffered Cursor writer fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"OK");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_buffered_writer_defers_target_write_until_flush() {
    let (root, input, output) = project(
        "buffered-writer-deferred",
        "open cursor;\nopen buffered;\nopen reader;\nopen writer;\nopen stdout;\nopen error;\n",
        &["cursor.act", "buffered.act", "reader.act", "writer.act", "stdout.act", "error.act"],
        "import io; verb measure(ins writer: BufferedWriter[Cursor], abs payload: Buffer) -> Int { buffered_write(writer: ins writer, input: abs payload); return writer.target.position; } verb main() -> Int { erg storage = Buffer[0]; erg target = Cursor { buffer: storage, position: 0, }; erg scratch = Buffer[0]; erg capacity = 4; reserve(buffer: ins scratch, capacity: capacity); erg writer: BufferedWriter[Cursor] = BufferedWriter[Cursor] { target: target, buffer: scratch, }; erg payload = Buffer[0]; append(payload, 79); append(payload, 75); erg before = measure(writer: ins writer, payload: abs payload); flush_buffer(writer: ins writer); return before; }\n",
    );
    build(&input, &output);
    let execution =
        Command::new(&output).output().expect("buffered writer deferred fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_copy_propagates_reader_failure() {
    let (root, input, output) = project(
        "copy-reader-error",
        "open cursor;\nopen copy;\nopen reader;\nopen writer;\nopen error;\n",
        &["cursor.act", "copy.act", "reader.act", "writer.act", "error.act"],
        "import io; struct BrokenReader { erg marker: Int, } perform Reader for BrokenReader { verb read(ins self: BrokenReader, ins buffer: Buffer) -> Result[Int, IoError] { return Result[Int, IoError].Err(IoError.Failed); } } verb main() -> Int { erg reader = BrokenReader { marker: 0, }; erg output = Buffer[0]; erg writer = Cursor { buffer: output, position: 0, }; erg result = copy_stream(reader: ins reader, writer: ins writer); return case dat result { Result.Err(error) => case dat error { IoError.Failed => 0, IoError.EndOfStream => 1, IoError.InvalidInput => 1, IoError.InvalidData => 1, }, Result.Ok(_) => 1, }; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("copy reader error fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_copy_propagates_writer_failure() {
    let (root, input, output) = project(
        "copy-writer-error",
        "open cursor;\nopen copy;\nopen reader;\nopen writer;\nopen error;\n",
        &["cursor.act", "copy.act", "reader.act", "writer.act", "error.act"],
        "import io; perform Writer for Int { verb write(ins self: Int, abs buffer: Buffer) -> Result[Int, IoError] { return Result[Int, IoError].Err(IoError.Failed); } verb flush(ins self: Int) -> Result[Int, IoError] { return Result[Int, IoError].Err(IoError.Failed); } } verb main() -> Int { erg input = Buffer[0]; append(input, 65); erg reader = Cursor { buffer: input, position: 0, }; erg writer = 0; erg result = copy_stream(reader: ins reader, writer: ins writer); return case dat result { Result.Err(error) => case dat error { IoError.Failed => 0, IoError.EndOfStream => 1, IoError.InvalidInput => 1, IoError.InvalidData => 1, }, Result.Ok(_) => 1, }; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("copy writer error fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_cursor_seek_rejects_out_of_bounds_positions() {
    let (root, input, output) = project(
        "cursor-seek",
        "open cursor;\nopen reader;\nopen writer;\nopen stdout;\nopen error;\n",
        &["cursor.act", "reader.act", "writer.act", "stdout.act", "error.act"],
        "import io; verb main() -> Int { erg bytes = Buffer[0]; append(bytes, 88); erg cursor = Cursor { buffer: bytes, position: 0, }; erg offset = 2; erg result = seek(self: ins cursor, offset: offset); return case dat result { Result.Ok(_) => 1, Result.Err(_) => 0, }; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("seek fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_copy_transfers_cursor_streams_until_eof() {
    let (root, input, output) = project(
        "cursor-copy",
        "open cursor;\nopen copy;\nopen reader;\nopen writer;\nopen stdout;\nopen error;\n",
        &["cursor.act", "copy.act", "reader.act", "writer.act", "stdout.act", "error.act"],
        "import io; verb main() -> Int { erg input = Buffer[0]; append(input, 65); append(input, 66); append(input, 67); erg source = Cursor { buffer: input, position: 0, }; erg output = Buffer[0]; erg target = Cursor { buffer: output, position: 0, }; copy_stream(reader: ins source, writer: ins target); erg offset = 0; seek(self: ins target, offset: offset); erg result = Buffer[0]; cursor_read(self: ins target, buffer: ins result); printb(text: abs result); flush(); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("copy fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"ABC");
    let _ = fs::remove_dir_all(root);
}

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use actus::cli::run_with_args;

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
        "import io; verb main() -> Int { erg storage = Buffer[0]; append(storage, 97); append(storage, 98); append(storage, 99); append(storage, 100); append(storage, 101); append(storage, 102); print(text: abs storage); flush(); return 0; }\n",
    );
    build(&input, &output);
    let mut child = Command::new(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("buffered fixture should run");
    child.stdin.take().expect("stdin pipe should exist").write_all(b"buffered\n").unwrap();
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
        "import io; verb main() -> Int { erg text = Buffer[0]; append(text, 72); append(text, 105); print(text: abs text); println(text: abs text); eprint(text: abs text); eprintln(text: abs text); flush(); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("text fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"HiHi\n");
    assert_eq!(execution.stderr, b"HiHi\n");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_bridge_reads_line_into_exclusive_buffer() {
    let (root, input, output) = project(
        "stdin",
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n",
        &["stdout.act", "stderr.act", "stdin.act", "error.act"],
        "import io; verb main() -> Int { erg buffer = Buffer[0]; read_line(buffer: ins buffer); print(text: abs buffer); flush(); return 0; }\n",
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
    let mut child = Command::new(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("stream fixture should run");
    child.stdin.take().expect("stream stdin should exist").write_all(b"stream\n").unwrap();
    let execution = child.wait_with_output().expect("wait for stream fixture");
    assert_eq!(execution.status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_cursor_reader_uses_static_generic_dispatch() {
    let (root, input, output) = project(
        "cursor-reader",
        "open cursor;\nopen buffered;\nopen reader;\nopen writer;\nopen stdout;\nopen error;\n",
        &["cursor.act", "buffered.act", "reader.act", "writer.act", "stdout.act", "error.act"],
        "import io; verb main() -> Int { erg payload = Buffer[0]; append(payload, 67); append(payload, 117); append(payload, 114); erg storage = Buffer[0]; actus_cursor_write(target: ins storage, source: abs payload); erg output = Buffer[0]; actus_cursor_read(source: abs storage, target: ins output); print(text: abs output); flush(); return 0; }\n",
    );
    build(&input, &output);
    let execution = Command::new(&output).output().expect("cursor fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"Cur");
    let _ = fs::remove_dir_all(root);
}

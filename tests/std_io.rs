use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use actus::cli::run_with_args;
use actus::lexer::scan;
use actus::modules::{ModuleResolver, analyze_module, exports_module};
use actus::parser::parse;
use actus::semantic::analyze;

fn library_source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/io").join(file);
    fs::read_to_string(path).expect("standard library source should exist")
}

#[test]
fn std_io_declarations_pass_semantic_validation() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let resolver = ModuleResolver::new(source_root);
    let exports = exports_module(&resolver, "io").expect("std io facade should resolve");
    assert!(exports.contains("verb", "print_int"));
    assert!(exports.contains("verb", "eprint_int"));
    assert!(exports.contains("verb", "print"));
    assert!(exports.contains("verb", "println"));
    assert!(exports.contains("verb", "eprint"));
    assert!(exports.contains("verb", "eprintln"));
    assert!(exports.contains("verb", "flush"));
    assert!(exports.contains("verb", "read_line"));
    assert!(exports.contains("verb", "read_byte"));
    assert!(exports.contains("verb", "read"));
    assert!(exports.contains("verb", "write"));
    assert!(exports.contains("struct", "BufferedReader"));
    assert!(exports.contains("struct", "BufferedWriter"));
    assert!(exports.contains("verb", "buffered_reader"));
    assert!(exports.contains("verb", "buffered_writer"));
    assert!(exports.contains("verb", "refill"));
    assert!(exports.contains("verb", "buffered_write"));
    assert!(exports.contains("enum", "IoError"));
    analyze_module(&resolver, "io").expect("std io declarations should be semantically valid");
}

#[test]
fn std_io_buffered_constructor_requires_dat_ownership_transfer() {
    let source = "struct BufferedReader { erg buffer: Buffer, } verb buffered_reader(dat buffer: Buffer) -> BufferedReader { return BufferedReader { buffer: buffer, }; } verb main() -> Int { erg buffer = Buffer[4]; erg reader = buffered_reader(buffer: abs buffer); return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("buffered fixture should parse");
    assert!(analyze(&program).is_err());
}

#[test]
fn std_io_native_buffered_reader_reuses_owned_buffer() {
    let root = std::env::temp_dir().join(format!("actus-std-buffered-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create buffered fixture");
    fs::write(
        io_root.join("io.act"),
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\nopen read;\nopen write;\nopen buffered;\n",
    )
    .expect("write buffered facade");
    for file in [
        "stdout.act",
        "stderr.act",
        "stdin.act",
        "error.act",
        "read.act",
        "write.act",
        "buffered.act",
    ] {
        fs::write(io_root.join(file), library_source(file)).expect("copy buffered source");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-buffered-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write buffered manifest");
    let input = source_root.join("main.act");
    fs::write(
        &input,
        "import io; verb main() -> Int { erg reader = BufferedReader { buffer: Buffer[0], }; erg writer = BufferedWriter { buffer: Buffer[0], }; erg payload = Buffer[0]; buffered_write(input: abs payload); return 0; }\n",
    )
    .expect("write buffered entry");
    let output = root.join("std-buffered");
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
    let mut child = Command::new(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("buffered fixture should run");
    child.stdin.take().expect("stdin pipe should exist").write_all(b"buffered\n").unwrap();
    let execution = child.wait_with_output().expect("wait for buffered fixture");
    assert_eq!(execution.status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_read_line_requires_an_explicit_ins_argument() {
    let source = "verb read_line(ins buffer: Buffer) -> Int { return 0; } verb main() -> Int { erg buffer = Buffer[0]; return read_line(buffer: buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("invalid ins call should parse");
    assert!(analyze(&program).is_err());
}

#[test]
fn std_io_native_bridge_separates_stdout_and_stderr() {
    let root = std::env::temp_dir().join(format!("actus-std-io-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create std io fixture");
    for file in ["io.act", "stdout.act", "stderr.act", "stdin.act", "error.act"] {
        fs::write(
            io_root.join(file),
            if file == "io.act" {
                "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n".to_owned()
            } else {
                library_source(file)
            },
        )
        .expect("copy std io source");
    }
    let input = source_root.join("main.act");
    let output = root.join("std-io");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-io-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write fixture manifest");
    fs::write(
        &input,
        "import io; verb main() -> Int { erg value = 42; print_int(value: value); eprint_int(value: value); return 0; }\n",
    )
    .expect("write std io entry");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);

    let execution = Command::new(&output).output().expect("std io fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"42\n");
    assert_eq!(execution.stderr, b"42\n");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_bridge_prints_borrowed_buffer_text() {
    let root = std::env::temp_dir().join(format!("actus-std-text-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create std text fixture");
    fs::write(io_root.join("io.act"), "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n")
        .expect("write io facade");
    for file in ["stdout.act", "stderr.act", "stdin.act", "error.act"] {
        fs::write(io_root.join(file), library_source(file)).expect("copy std io source");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-text-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write text fixture manifest");
    let input = source_root.join("main.act");
    fs::write(
        &input,
        "import io; verb main() -> Int { erg text = Buffer[0]; append(text, 72); append(text, 105); print(text: abs text); println(text: abs text); eprint(text: abs text); eprintln(text: abs text); flush(); return 0; }\n",
    )
    .expect("write text fixture entry");
    let output = root.join("std-text");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let execution = Command::new(&output).output().expect("text fixture should run");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"HiHi\n");
    assert_eq!(execution.stderr, b"HiHi\n");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_native_bridge_reads_line_into_exclusive_buffer() {
    let root = std::env::temp_dir().join(format!("actus-std-stdin-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create stdin fixture");
    fs::write(io_root.join("io.act"), "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n")
        .expect("write io facade");
    for file in ["stdout.act", "stderr.act", "stdin.act", "error.act"] {
        fs::write(io_root.join(file), library_source(file)).expect("copy std io source");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-stdin-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write stdin fixture manifest");
    let input = source_root.join("main.act");
    fs::write(
        &input,
        "import io; verb main() -> Int { erg buffer = Buffer[0]; read_line(buffer: ins buffer); print(text: abs buffer); flush(); return 0; }\n",
    )
    .expect("write stdin fixture entry");
    let output = root.join("std-stdin");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
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
    let root = std::env::temp_dir().join(format!("actus-std-byte-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create byte fixture");
    fs::write(io_root.join("io.act"), "open stdout;\nopen stderr;\nopen stdin;\nopen error;\n")
        .expect("write io facade");
    for file in ["stdout.act", "stderr.act", "stdin.act", "error.act"] {
        fs::write(io_root.join(file), library_source(file)).expect("copy std io source");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-byte-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write byte fixture manifest");
    let input = source_root.join("main.act");
    fs::write(
        &input,
        "import io; verb consume_byte() -> Result[Int, IoError] { read_byte()?; return Result[Int, IoError].Ok(0); } verb main() -> Int { consume_byte(); return 42; }\n",
    )
        .expect("write byte fixture entry");
    let output = root.join("std-byte");
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
    let execution = Command::new(&output).output().expect("byte fixture should run");
    assert_eq!(execution.status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn std_io_stream_result_chain_propagates_success() {
    let root = std::env::temp_dir().join(format!("actus-std-stream-{}", std::process::id()));
    let source_root = root.join("src");
    let io_root = source_root.join("io");
    fs::create_dir_all(&io_root).expect("create stream fixture");
    fs::write(
        io_root.join("io.act"),
        "open stdout;\nopen stderr;\nopen stdin;\nopen error;\nopen read;\nopen write;\n",
    )
    .expect("write stream facade");
    for file in ["stdout.act", "stderr.act", "stdin.act", "error.act", "read.act", "write.act"] {
        fs::write(io_root.join(file), library_source(file)).expect("copy stream source");
    }
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std-stream-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write stream manifest");
    let input = source_root.join("main.act");
    fs::write(
        &input,
        "import io; verb stream() -> Result[Int, IoError] { erg buffer = Buffer[0]; append(buffer, 115); append(buffer, 116); append(buffer, 114); append(buffer, 101); append(buffer, 97); append(buffer, 109); write(buffer: abs buffer)?; return Result[Int, IoError].Ok(0); } verb main() -> Int { stream(); return 42; }\n",
    )
    .expect("write stream entry");
    let output = root.join("std-stream");
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

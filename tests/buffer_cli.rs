use std::fs;

use actus::cli::run_with_args;

#[cfg(unix)]
fn build_and_run(source: &str, artifact: &str) -> i32 {
    let root = std::env::temp_dir().join(format!("actus-cli-{artifact}-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, source).expect("write source");
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
    let status = std::process::Command::new(&output).status().expect("run executable");
    let code = status.code().expect("process exit code");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    code
}

#[cfg(unix)]
#[test]
fn links_buffer_runtime_operations() {
    let source = "verb main() -> Int { erg buffer: Buffer = allocate(4); append(buffer, 42); drop(buffer); return 42; }\n";
    assert_eq!(build_and_run(source, "buffer"), 42);
}

#[cfg(unix)]
#[test]
fn drops_buffer_on_return_unwind() {
    let source = "verb main() -> Int { erg buffer: Buffer = allocate(4); return 42; }\n";
    assert_eq!(build_and_run(source, "buffer-return"), 42);
}

#[cfg(unix)]
#[test]
fn transfers_buffer_return_ownership() {
    let source = "verb create() -> Buffer { erg buffer: Buffer = allocate(4); return buffer; } verb main() -> Int { erg buffer: Buffer = create(); append(buffer, 42); return 42; }\n";
    assert_eq!(build_and_run(source, "buffer-return-owner"), 42);
}

#[cfg(unix)]
#[test]
fn transfers_dat_buffer_without_duplicate_cleanup() {
    let source = "verb consume(dat buffer: Buffer) -> Int { return 42; } verb main() -> Int { erg buffer: Buffer = allocate(4); return consume(buffer: buffer); }\n";
    assert_eq!(build_and_run(source, "buffer-dat"), 42);
}

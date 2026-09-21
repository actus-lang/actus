use std::fs;

use actus::cli::run_with_args;

#[test]
fn build_command_writes_a_native_object() {
    let root = std::env::temp_dir().join(format!("actus-cli-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("o");
    fs::write(
        &input,
        "verb main() -> Int { erg left = 40; erg right = 2; return add(left: left, right: right); } verb add(erg left: Int, erg right: Int) -> Int { return left + right; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let object = fs::read(&output).expect("read object");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xfe\xed"));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn run_command_builds_and_executes_source() {
    let input = std::env::temp_dir().join(format!("actus-cli-run-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

    let result = run_with_args(vec!["run".to_owned(), input.display().to_string()].into_iter());

    assert_eq!(result, 42);
    let _ = fs::remove_file(input);
}

#[cfg(unix)]
#[test]
fn build_command_links_an_executable() {
    let root = std::env::temp_dir().join(format!("actus-cli-exe-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg left = 40; erg right = 2; return add(left: left, right: right); } verb add(erg left: Int, erg right: Int) -> Int { return left + right; }\n",
    )
    .expect("write source");

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
    assert_eq!(status.code(), Some(42));
    assert!(!output.with_extension("o").exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_unary_and_grouped_expression() {
    let root = std::env::temp_dir().join(format!("actus-cli-expression-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { return -(2 + 3) * -4; }\n").expect("write source");

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
    assert_eq!(status.code(), Some(20));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_nested_scalar_scope_and_drop() {
    let root = std::env::temp_dir().join(format!("actus-cli-scope-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 41; { abs view = ref value; } drop(value); return 42; }\n",
    )
    .expect("write source");

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
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_unwinds_borrow_scope_on_early_return() {
    let root = std::env::temp_dir().join(format!("actus-cli-borrow-return-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 41; { abs view = ref value; return view + 1; } }\n",
    )
    .expect("write source");

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
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_scalar_dat_transfer_without_duplicate_cleanup() {
    let root = std::env::temp_dir().join(format!("actus-cli-dat-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 41; return consume(value: value); } verb consume(dat value: Int) -> Int { return value + 1; }\n",
    )
    .expect("write source");

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
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_links_buffer_runtime_operations() {
    let root = std::env::temp_dir().join(format!("actus-cli-buffer-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg buffer: Buffer = allocate(4); append(buffer, 42); drop(buffer); return 42; }\n",
    )
    .expect("write source");

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
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_drops_buffer_on_return_unwind() {
    let root = std::env::temp_dir().join(format!("actus-cli-buffer-return-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { erg buffer: Buffer = allocate(4); return 42; }\n")
        .expect("write source");

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
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_loop_break_control_flow() {
    let root = std::env::temp_dir().join(format!("actus-cli-loop-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { loop { break; } return 42; }\n").expect("write source");

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
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_return_from_loop_body() {
    let root = std::env::temp_dir().join(format!("actus-cli-loop-return-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { loop { return 42; } }\n").expect("write source");

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
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_preserves_loop_carried_binding_values() {
    let root = std::env::temp_dir().join(format!("actus-cli-loop-ssa-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 0; loop { value = 1; break; } return value; }\n",
    )
    .expect("write source");

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
    assert_eq!(status.code(), Some(1));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn check_command_rejects_semantically_invalid_source() {
    let input = std::env::temp_dir().join(format!("actus-cli-check-{}.act", std::process::id()));
    fs::write(&input, "verb main() { inspect(missing); }\n").expect("write source");

    let result = run_with_args(vec!["check".to_owned(), input.display().to_string()].into_iter());

    assert_eq!(result, 1);
    let _ = fs::remove_file(input);
}

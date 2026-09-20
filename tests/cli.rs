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

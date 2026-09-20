use std::fs;

use actus::cli::run_with_args;

#[test]
fn build_command_writes_a_native_object() {
    let root = std::env::temp_dir().join(format!("actus-cli-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("o");
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

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

#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
#[test]
fn executes_width_qualified_integer_float_and_void_abi_paths() {
    let root = std::env::temp_dir().join(format!("actus-primitive-types-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb echo_byte(erg value: u8) -> u8 { return value; } verb echo_word(erg value: u16) -> u16 { return value; } verb echo_dword(erg value: u32) -> u32 { return value; } verb echo_qword(erg value: u64) -> u64 { return value; } verb echo_wide(erg value: u128) -> u128 { return value; } verb echo_signed(erg value: i8) -> i8 { return value; } verb echo_float(erg value: f32) -> f32 { return value; } verb echo_double(erg value: f64) -> f64 { return value; } verb noop() -> Void { return; } verb main() -> Int { erg byte: u8 = 255; erg word: u16 = 255; erg dword: u32 = 255; erg qword: u64 = 255; erg signed: i8 = -1; erg sample: f32 = 1.5; erg double_sample: f64 = 1.5; erg echoed_byte: u8 = echo_byte(value: byte); erg echoed_word: u16 = echo_word(value: word); erg echoed_dword: u32 = echo_dword(value: dword); erg echoed_qword: u64 = echo_qword(value: qword); erg echoed_signed: i8 = echo_signed(value: signed); erg returned_float: f32 = echo_float(value: sample); erg returned_double: f64 = echo_double(value: double_sample); noop(); return 0; }\n",
    )
    .expect("write primitive fixture");
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
    let status = std::process::Command::new(&output).status().expect("run primitive fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

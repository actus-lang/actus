use std::fs;

use actus::cli::run_with_args;

#[cfg(unix)]
fn build_and_run(source: &str, name: &str, expected: i32) {
    let root = std::env::temp_dir().join(format!("actus-cli-{name}-{}", std::process::id()));
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
    assert_eq!(status.code(), Some(expected));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn executes_struct_field_access() {
    build_and_run(
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg point = Point { x: 40, y: 2, }; return point.x + point.y; }\n",
        "struct-access",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_struct_field_assignment() {
    build_and_run(
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg point = Point { x: 40, y: 2, }; point.x = 41; return point.x + point.y; }\n",
        "struct-assignment",
        43,
    );
}

#[cfg(unix)]
#[test]
fn executes_nested_struct_access_and_owned_drop() {
    build_and_run(
        "struct Inner { x: Int, y: Int, } struct Holder { inner: Inner, erg payload: Buffer, } verb main() -> Int { erg holder = Holder { inner: Inner { x: 40, y: 2, }, payload: allocate(4), }; return holder.inner.x + holder.inner.y; }\n",
        "nested-struct",
        42,
    );
}

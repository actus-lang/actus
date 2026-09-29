#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
#[test]
fn executes_contiguous_array_reads_and_writes_natively() {
    let status = run_array_fixture(
        "array-read-write",
        "verb main() -> Int { erg values: Array[Int, 4] = Array[Int, 4](); values[1] = 41; return values[1] + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn traps_deterministically_on_dynamic_array_bounds_failure() {
    let status = run_array_fixture(
        "array-bounds",
        "verb main() -> Int { erg values: Array[Int, 4] = Array[Int, 4](); erg index = 4; return values[index]; }",
    );
    assert!(!status.success());
    assert!(status.code().is_none(), "bounds failure should be a native trap: {status:?}");
}

#[cfg(unix)]
#[test]
fn copies_aggregate_array_elements_without_copying_the_array() {
    let status = run_array_fixture(
        "array-aggregate",
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg points: Array[Point, 2] = Array[Point, 2](); points[1] = Point { x: 19, y: 23, }; return points[1].x + points[1].y; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn places_contiguous_arrays_in_an_arena_without_reallocation() {
    let status = run_array_fixture(
        "array-arena",
        "verb main() -> Int { erg arena: Arena[128] = Arena[128](); erg values: Array[Int, 4] = arena.place(value: Array[Int, 4]()); values[2] = 41; return values[2] + 1; }",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn mutates_indexed_pack_fields_in_place() {
    let status = run_array_fixture(
        "array-pack",
        "pack Control { erg storage: u8; layout little; fields { erg enabled: u1 at 0; abs _reserved: u7 at 1; } } verb main() -> Int { erg controls: Array[Control, 2] = Array[Control, 2](); controls[1] = Control { storage: 0, }; controls[1].enabled = 1; return controls[1].enabled; }",
    );
    assert_eq!(status.code(), Some(1));
}

#[cfg(unix)]
#[test]
fn emits_identical_objects_for_repeated_array_builds() {
    let root = std::env::temp_dir().join(format!("actus-array-repeat-{}", std::process::id()));
    let input = root.with_extension("act");
    let first = root.with_extension("first.o");
    let second = root.with_extension("second.o");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[Int, 2] = Array[Int, 2](); values[1] = 41; return values[1] + 1; }",
    )
    .expect("write deterministic array fixture");
    for output in [&first, &second] {
        let result = run_with_args(
            vec![
                "build".to_owned(),
                input.display().to_string(),
                "--emit".to_owned(),
                "obj".to_owned(),
                "-o".to_owned(),
                output.display().to_string(),
            ]
            .into_iter(),
        );
        assert_eq!(result, 0);
    }
    assert_eq!(
        fs::read(&first).expect("read first object"),
        fs::read(&second).expect("read second object")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[cfg(unix)]
fn run_array_fixture(name: &str, source: &str) -> std::process::ExitStatus {
    let root = std::env::temp_dir().join(format!("actus-{name}-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, source).expect("write array fixture");
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
    let status = std::process::Command::new(&output).status().expect("run array fixture");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    status
}

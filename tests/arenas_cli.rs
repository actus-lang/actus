#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
fn build_and_run(source: &str, label: &str) -> std::process::ExitStatus {
    let root = std::env::temp_dir().join(format!("actus-arena-{label}-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, source).expect("write arena fixture");
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
    let status = std::process::Command::new(&output).status().expect("run arena fixture");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    status
}

#[cfg(unix)]
#[test]
fn places_multiple_nodes_with_aligned_bump_pointer_storage() {
    let status = build_and_run(
        "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[64] = Arena[64](); erg first: Node = arena.place(value: Node { value: 20, }); erg second: Node = arena.place(value: Node { value: 22, }); return first.value + second.value; }",
        "tree",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
#[test]
fn traps_deterministically_when_arena_capacity_is_exhausted() {
    let status = build_and_run(
        "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[4] = Arena[4](); erg first: Node = arena.place(value: Node { value: 1, }); erg second: Node = arena.place(value: Node { value: 2, }); return first.value + second.value; }",
        "overflow",
    );
    assert!(!status.success(), "arena overflow must terminate through a native trap");
}

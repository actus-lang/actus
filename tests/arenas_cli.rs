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

#[cfg(unix)]
fn check_arena_program(body: &str, label: &str) -> i32 {
    let input = std::env::temp_dir()
        .join(format!("actus-arena-check-{label}-{}", std::process::id()))
        .with_extension("act");
    let prelude = "struct Node { value: Int, } verb take(dat arena: Arena[64]) {} ";
    fs::write(&input, format!("{prelude}verb main() {{ {body} }}")).expect("write arena fixture");
    let result = run_with_args(vec!["check".to_owned(), input.display().to_string()].into_iter());
    let _ = fs::remove_file(input);
    result
}

#[cfg(unix)]
#[test]
fn check_rejects_an_arena_moved_while_a_view_is_live() {
    let place = "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); abs view = ref node; ";
    for (label, consume) in
        [("take", "take(arena: arena);"), ("bind", "erg other = arena;"), ("drop", "drop(arena);")]
    {
        for body in [
            format!("{place}{consume} print(view.value);"),
            format!("{place}print(view.value); {consume}"),
        ] {
            let result = check_arena_program(&body, label);
            assert_ne!(result, 0, "{label}: a moved arena with a live view must fail `check`");
        }
    }
}

#[cfg(unix)]
#[test]
fn check_accepts_an_arena_released_after_its_view_block_ends() {
    let place =
        "erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 1, }); ";
    for (label, consume) in [("take", "take(arena: arena);"), ("drop", "drop(arena);")] {
        let result = check_arena_program(
            &format!("{place}{{ abs view = ref node; print(view.value); }} {consume}"),
            label,
        );
        assert_eq!(result, 0, "{label}: a view in a closed block must not block the release");
    }
}

#[cfg(unix)]
#[test]
fn runs_a_program_that_drops_its_arena_after_the_view_block() {
    let status = build_and_run(
        "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[64] = Arena[64](); erg node = arena.place(value: Node { value: 42, }); erg seen: Int = 0; { abs view = ref node; seen = view.value; } drop(arena); return seen; }",
        "closed-view",
    );
    assert_eq!(status.code(), Some(42));
}

#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
#[test]
fn executes_arena_tree_through_indirect_reference_fields() {
    let root = std::env::temp_dir().join(format!("actus-arena-graph-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = r#"
struct Leaf { value: Int, }
struct Node { value: Int, abs left: Leaf, abs right: Leaf, }

verb main() -> Int {
    erg arena: Arena[128] = Arena[128]();
    erg left = arena.place(Leaf { value: 10, });
    erg right = arena.place(Leaf { value: 20, });
    erg root = arena.place(Node { value: 12, left: left, right: right, });
    return root.value + root.left.value + root.right.value;
}
"#;
    fs::write(&input, source).expect("write graph fixture");
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
    let status = std::process::Command::new(&output).status().expect("run graph fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

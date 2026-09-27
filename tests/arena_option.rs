#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

#[test]
fn accepts_option_reference_payloads_for_recursive_nodes() {
    analyze_source(
        "struct ListNode { value: Int, next: Option[abs ListNode], } verb main() -> Int { return 0; }",
    )
    .expect("Option[abs Node] should be a valid recursive type");
}

#[test]
fn rejects_arena_drop_while_an_option_reference_is_live() {
    let error = analyze_source(
        "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[128] = Arena[128](); erg node = arena.place(value: Node { value: 1, }); abs view = ref node; drop(arena); return view.value; }",
    )
    .expect_err("an arena owner must not be dropped while its view is live");
    assert!(
        matches!(error, SemanticErrorKind::ArenaReferenceLive { arena, reference } if arena == "arena" && reference == "view")
    );
}

#[cfg(unix)]
#[test]
fn executes_recursive_option_list_with_pointer_niche() {
    let root = std::env::temp_dir().join(format!("actus-arena-option-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = r#"
struct ListNode {
    value: Int,
    next: Option[abs ListNode],
}

verb sum(abs node: ListNode) -> Int {
    abs next = ref node.next;
    return node.value + case next {
        Option.Some(child) => sum(node: abs child),
        Option.None => 0,
    };
}

verb main() -> Int {
    erg arena: Arena[256] = Arena[256]();
    erg tail = arena.place(value: ListNode { value: 30, next: Option[abs ListNode].None, });
    erg middle = arena.place(value: ListNode { value: 20, next: Option[abs ListNode].Some(tail), });
    erg head = arena.place(value: ListNode { value: 10, next: Option[abs ListNode].Some(middle), });
    abs root = ref head;
    return sum(node: abs root);
}
"#;
    fs::write(&input, source).expect("write recursive option fixture");
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
    let status = std::process::Command::new(&output).status().expect("run option fixture");
    assert_eq!(status.code(), Some(60));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

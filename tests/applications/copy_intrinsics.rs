use super::support::{build, project, run};
use std::fs;

#[test]
fn imported_io_copy_does_not_capture_scalar_copy_intrinsic() {
    let source = r#"import io;

verb scalar_copy() -> Int {
    erg value: u32 = 41u32;
    erg reused: u32 = copy(value: abs value);
    return reused as Int;
}

verb main() -> Int {
    return scalar_copy();
}
"#;
    let (root, input, output) = project("imported-io-copy-intrinsic", source);
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove imported io copy project");
}

#[test]
fn nested_facade_keeps_scalar_copy_intrinsic_separate_from_imported_io_copy() {
    let source = "import feature; verb main() -> Int { return scalar_copy(); }\n";
    let (root, input, output) = project("nested-imported-io-copy-intrinsic", source);
    fs::create_dir_all(root.join("src/feature")).expect("create feature module");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import io;

open struct Storage[N: Usize] {
    erg value: u32,
}

open verb configure(ins storage: Storage[64], erg index: u32) -> u32 {
    storage.value = index;
    return index;
}

open verb produce[N: Usize](ins storage: Storage[N]) -> Option[u32] {
    return Option[u32].Some(41u32);
}

open verb scalar_copy() -> Int {
    erg storage: Storage[64] = Storage[64] { value: 0u32, };
    erg created: Option[u32] = produce(storage: ins storage);
    erg selected: u32 = case dat created {
        Option.Some(index) => configure(storage: ins storage, index: erg index),
        Option.None => 0u32,
    };
    erg reused: u32 = copy(value: abs selected);
    return reused as Int;
}
"#,
    )
    .expect("write feature implementation");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(41));
    fs::remove_dir_all(root).expect("remove nested imported io copy project");
}

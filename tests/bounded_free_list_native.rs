#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
const SOURCE: &str = r#"
const CAPACITY: u32 = 8u32;
const NULL_INDEX: u32 = 4294967295u32;

enum LinkError {
    Capacity,
}

struct Table[N: Usize] {
    erg links: Array[u32, 8],
}

verb take_record[N: Usize](ins table: Table[N]) -> Option[u32] {
    if table.links[0u32] == NULL_INDEX {
        return Option[u32].None;
    }
    erg index: u32 = table.links[0u32];
    table.links[0u32] = table.links[index];
    table.links[index] = NULL_INDEX;
    return Option[u32].Some(index);
}

verb acquire[N: Usize](ins table: Table[N]) -> Result[u32, LinkError] {
    erg allocated = take_record[N](table: ins table);
    erg record_index: u32 = case dat allocated {
        Option.None => NULL_INDEX,
        Option.Some(value) => value,
    };
    if record_index == NULL_INDEX || record_index >= CAPACITY {
        return Result[u32, LinkError].Err(LinkError.Capacity);
    }
    return Result[u32, LinkError].Ok(record_index);
}

verb main() -> Int {
    erg table: Table[64] = Table[64] { links: Array[u32, 8](), };
    table.links[0u32] = 1u32;
    table.links[1u32] = 2u32;
    table.links[2u32] = NULL_INDEX;
    erg acquired = acquire[64](table: ins table);
    return case dat acquired {
        Result.Err(_) => 1,
        Result.Ok(value) => if value == 1u32 { 0 } else { 2 },
    };
}
"#;

#[cfg(unix)]
fn run_fixture(backend: &str) {
    let root = std::env::temp_dir()
        .join(format!("actus-bounded-free-list-{backend}-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, SOURCE).expect("write bounded free-list fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--strict".to_owned(),
            "--backend".to_owned(),
            backend.to_owned(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0, "bounded free-list fixture should build on {backend}");
    let status =
        std::process::Command::new(&output).status().expect("run bounded free-list fixture");
    assert_eq!(status.code(), Some(0), "bounded free-list fixture failed on {backend}");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn bounded_free_list_option_result_path_runs_on_cranelift() {
    run_fixture("cranelift");
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn bounded_free_list_option_result_path_runs_on_llvm() {
    run_fixture("llvm");
}

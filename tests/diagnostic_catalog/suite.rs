//! The check of ADR-0123, "The catalog is checked".
//!
//! 1. every code of the registry has an entry or is on `missing.txt`, and
//!    every entry and listed code is in the registry; a retired code is
//!    neither required nor allowed as an entry or a listed code, and
//!    `diagnostics.md` names each retired code;
//! 2. every `fails=` example reports its code first, inside the example;
//! 3. every `compiles` group passes `actus check`;
//! 4. every entry has the documented shape.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use actus::diagnostics::DiagnosticCode;

use super::entry::{Block, Entry, is_code, parse_entry};
use super::project::{FirstError, Run, TempProject, first_error, groups, is_expected};

fn catalog_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/language/diagnostics")
}

/// Every entry of the catalog, parsed; every problem is collected.
fn load_entries() -> (Vec<Entry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut problems = Vec::new();
    let listing = fs::read_dir(catalog_directory()).expect("the catalog directory should exist");
    let mut names: Vec<String> = listing
        .map(|item| item.expect("directory entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names.iter().filter(|name| name.as_str() != "missing.txt") {
        let Some(code) = name.strip_suffix(".md").filter(|code| is_code(code)) else {
            problems
                .push(format!("{name}: the catalog holds only `E####.md` files and missing.txt"));
            continue;
        };
        let text = fs::read_to_string(catalog_directory().join(name)).expect("entry is readable");
        match parse_entry(code, &text) {
            Ok(entry) => entries.push(entry),
            Err(problem) => problems.push(format!("{name}: {problem}")),
        }
    }
    (entries, problems)
}

fn assert_no_problems(problems: &[String]) {
    assert!(problems.is_empty(), "catalog problems:\n{}", problems.join("\n"));
}

#[test]
fn every_retired_code_is_named_in_the_diagnostics_document() {
    let document = fs::read_to_string(catalog_directory().with_file_name("diagnostics.md"))
        .expect("the diagnostics document should exist");
    let section = document
        .split("## Retired codes")
        .nth(1)
        .expect("diagnostics.md must have a `## Retired codes` section");
    let missing: Vec<&str> = DiagnosticCode::RETIRED
        .iter()
        .map(DiagnosticCode::as_str)
        .filter(|code| !section.contains(&format!("`{code}`")))
        .collect();
    assert!(missing.is_empty(), "retired codes missing from `## Retired codes`: {missing:?}");
}

#[test]
fn every_entry_has_the_documented_shape() {
    assert_no_problems(&load_entries().1);
}

#[test]
fn the_catalog_and_the_registry_list_the_same_codes() {
    let registry: BTreeSet<&str> = DiagnosticCode::ALL.iter().map(DiagnosticCode::as_str).collect();
    let listing = fs::read_to_string(catalog_directory().join("missing.txt")).unwrap_or_default();
    let listed: Vec<&str> =
        listing.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    let mut problems = Vec::new();
    if listed.windows(2).any(|pair| pair[0] >= pair[1]) {
        problems.push("missing.txt must be sorted, one code per line, without repeats".to_owned());
    }
    let (entries, _) = load_entries();
    let entered: BTreeSet<&str> = entries.iter().map(|entry| entry.code.as_str()).collect();
    let retired: BTreeSet<&str> =
        DiagnosticCode::RETIRED.iter().map(DiagnosticCode::as_str).collect();
    for code in &listed {
        if retired.contains(code) {
            problems.push(format!("{code}: is retired, remove it from missing.txt"));
        } else if !registry.contains(code) {
            problems.push(format!("{code}: listed in missing.txt but not in the registry"));
        } else if entered.contains(code) {
            problems.push(format!("{code}: has an entry, remove it from missing.txt"));
        }
    }
    for code in &entered {
        if retired.contains(code) {
            problems.push(format!("{code}: is retired, delete its entry"));
        } else if !registry.contains(code) {
            problems.push(format!("{code}: has an entry but is not in the registry"));
        }
    }
    for code in &registry {
        if !entered.contains(code) && !listed.contains(code) {
            problems.push(format!("{code}: has no entry and is not in missing.txt"));
        }
    }
    assert_no_problems(&problems);
}

#[test]
fn every_failing_example_reports_its_code_first_inside_the_example() {
    let (entries, _) = load_entries();
    let mut problems = Vec::new();
    let mut skipped = 0;
    for entry in &entries {
        let examples: Vec<_> =
            entry.blocks.iter().filter(|block| block.section == "Example").collect();
        for group in groups(&examples) {
            let Some(failing) = group.iter().find(|block| block.fails.is_some()) else {
                continue;
            };
            if skipped_without_llvm(&group, &mut skipped) {
                continue;
            }
            let command = Block::command_for(&group);
            let strict = Block::strict_for(&group);
            let project = TempProject::create(&group).expect("project should be created");
            let run = project.run(command, strict);
            let files: Vec<&str> = group.iter().map(|block| block.target()).collect();
            match first_error(&run) {
                Some(error) if is_expected(&error, &entry.code, &files) => {}
                Some(FirstError { code, location }) => problems.push(format!(
                    "{}: line {}: `actus {command}{}` reported {code} at `{}`, expected {} in one of {files:?}\n{}",
                    entry.code,
                    failing.line,
                    if strict { " --strict" } else { "" },
                    location.as_deref().unwrap_or("no file"),
                    entry.code,
                    run.output
                )),
                None => problems.push(format!(
                    "{}: line {}: `actus {command}` reported no error\n{}",
                    entry.code, failing.line, run.output
                )),
            }
        }
    }
    report_skipped(skipped);
    assert_no_problems(&problems);
}

#[test]
fn every_compiling_block_passes_check() {
    let (entries, _) = load_entries();
    let mut problems = Vec::new();
    let mut skipped = 0;
    for entry in &entries {
        let fixes: Vec<_> = entry.blocks.iter().filter(|block| block.section == "Fix").collect();
        for group in groups(&fixes) {
            let Some(compiling) = group.iter().find(|block| block.compiles) else {
                continue;
            };
            if skipped_without_llvm(&group, &mut skipped) {
                continue;
            }
            let project = TempProject::create(&group).expect("project should be created");
            let command = Block::command_for(&group);
            let strict = Block::strict_for(&group);
            let run = project.run(command, strict);
            if !run.success {
                problems.push(format!(
                    "{}: line {}: the block does not pass `actus {command}{}`\n{}",
                    entry.code,
                    compiling.line,
                    if strict { " --strict" } else { "" },
                    run.output
                ));
            }
        }
    }
    report_skipped(skipped);
    assert_no_problems(&problems);
}

/// Whether a group marked `llvm` is skipped because this test binary is built
/// without the `llvm` feature (ADR-0123); a skipped group is counted.
fn skipped_without_llvm(group: &[&Block], skipped: &mut usize) -> bool {
    let skip = Block::needs_llvm(group) && !cfg!(feature = "llvm");
    *skipped += usize::from(skip);
    skip
}

/// Prints how many groups were skipped, so a skip is visible (shown with
/// `--nocapture`, and always when the test fails).
fn report_skipped(skipped: usize) {
    if skipped > 0 {
        eprintln!(
            "diagnostic catalog: skipped {skipped} group(s) marked `llvm`, built without the `llvm` feature"
        );
    }
}

const VALID_ENTRY: &str = "# E1140: operands of different integer types\n\nExplanation.\n\n## Example\n\n```actus fails=E1140\nverb main() {}\n```\n\n## Fix\n\nText.\n\n```actus compiles\nverb main() {}\n```\n";

#[test]
fn a_well_formed_entry_is_accepted() {
    let entry = parse_entry("E1140", VALID_ENTRY).expect("a valid entry should parse");
    assert_eq!(entry.blocks.len(), 2);
}

#[test]
fn a_title_with_a_long_summary_is_rejected() {
    let long = "x".repeat(61);
    let text = VALID_ENTRY.replace("operands of different integer types", &long);
    assert!(parse_entry("E1140", &text).is_err());
}

#[test]
fn a_summary_that_starts_in_upper_case_is_rejected() {
    let text = VALID_ENTRY.replace("operands of", "Operands of");
    assert!(parse_entry("E1140", &text).is_err());
}

#[test]
fn an_entry_without_a_fix_is_rejected() {
    let text = VALID_ENTRY.replace("## Fix", "## Repair");
    assert!(parse_entry("E1140", &text).is_err());
}

#[test]
fn an_example_that_names_another_code_is_rejected() {
    let text = VALID_ENTRY.replace("fails=E1140", "fails=E1141");
    assert!(parse_entry("E1140", &text).is_err());
}

#[test]
fn the_test_marker_is_accepted_and_selects_actus_test() {
    let text = VALID_ENTRY.replace("fails=E1140", "fails=E1140,test");
    let entry = parse_entry("E1140", &text).expect("the `,test` marker should parse");
    let group: Vec<&Block> = entry.blocks.iter().take(1).collect();
    assert_eq!(Block::command_for(&group), "test");
}

#[test]
fn a_group_with_a_test_file_is_verified_by_actus_test() {
    let text = VALID_ENTRY.replace(
        "```actus compiles\nverb main() {}",
        "```actus compiles file=tests/main_test.act\nverb main() {}",
    );
    let entry = parse_entry("E1140", &text).expect("entry should parse");
    let fix: Vec<&Block> = entry.blocks.iter().filter(|block| block.compiles).collect();
    assert_eq!(Block::command_for(&fix), "test");
    let plain: Vec<&Block> = entry.blocks.iter().filter(|block| block.fails.is_some()).collect();
    assert_eq!(Block::command_for(&plain), "check");
}

#[test]
fn a_failing_example_keeps_the_command_of_its_marker_even_with_a_test_file() {
    let text = VALID_ENTRY.replace(
        "```actus fails=E1140\nverb main() {}",
        "```actus fails=E1140 file=tests/main_test.act\nverb main() {}",
    );
    let entry = parse_entry("E1140", &text).expect("entry should parse");
    let failing: Vec<&Block> = entry.blocks.iter().filter(|block| block.fails.is_some()).collect();
    assert_eq!(Block::command_for(&failing), "check");
}

#[test]
fn an_unknown_marker_is_rejected() {
    let text = VALID_ENTRY.replace("actus compiles", "actus compiles typo");
    assert!(parse_entry("E1140", &text).is_err());
}

fn strict_flags(text: &str) -> (bool, bool, bool) {
    let entry = parse_entry("E1140", text).expect("entry should parse");
    let failing: Vec<&Block> = entry.blocks.iter().filter(|block| block.fails.is_some()).collect();
    let fix: Vec<&Block> = entry.blocks.iter().filter(|block| block.compiles).collect();
    (Block::strict_for(&failing), Block::strict_for(&fix), failing[0].build)
}

#[test]
fn the_strict_marker_is_accepted_combined_and_selects_the_strict_run() {
    assert_eq!(strict_flags(VALID_ENTRY), (false, false, false));
    let strict = VALID_ENTRY.replace("fails=E1140", "fails=E1140,strict");
    assert_eq!(strict_flags(&strict), (true, false, false));
    let build = VALID_ENTRY.replace("fails=E1140", "fails=E1140,build,strict");
    assert_eq!(strict_flags(&build), (true, false, true));
    let fix = VALID_ENTRY.replace("actus compiles", "actus compiles strict");
    assert_eq!(strict_flags(&fix), (false, true, false));
}

#[test]
fn a_misplaced_strict_marker_is_rejected() {
    for text in [
        VALID_ENTRY.replace("fails=E1140", "fails=E1140,strict,build"),
        VALID_ENTRY.replace("fails=E1140", "fails=E1140,build,test"),
        VALID_ENTRY.replace("fails=E1140", "fails=E1140 strict"),
        VALID_ENTRY.replace("actus compiles", "actus compiles,strict"),
    ] {
        assert!(parse_entry("E1140", &text).is_err(), "{text}");
    }
    let alone = VALID_ENTRY.replace("```actus compiles\n", "```actus strict file=src/other.act\n");
    assert!(parse_entry("E1140", &alone).is_err());
}

fn run_of(output: &str) -> Run {
    Run { success: false, output: output.to_owned() }
}

#[test]
fn the_first_error_is_read_from_the_one_diagnostic_format() {
    let located = first_error(&run_of("src/main.act: error[E1140] at 3:5: operands\n"))
        .expect("a located error line");
    assert_eq!(
        (located.code.as_str(), located.location.as_deref()),
        ("E1140", Some("src/main.act"))
    );
    let manifest = first_error(&run_of("a/Actus.toml: error[E1801]: no location\n"))
        .expect("a manifest error line without a location");
    assert_eq!(
        (manifest.code.as_str(), manifest.location.as_deref()),
        ("E1801", Some("a/Actus.toml"))
    );
    let environment = first_error(&run_of("noise\nerror[E1806]: strict LLVM build\n"))
        .expect("an error line without a file");
    assert_eq!((environment.code.as_str(), environment.location), ("E1806", None));
    assert!(first_error(&run_of("error: cannot read\nwarning[E1850]: x\n")).is_none());
}

#[test]
fn a_line_without_a_file_is_accepted_only_for_an_environment_code() {
    let files = ["src/main.act"];
    let without_file = |code: &str| FirstError { code: code.to_owned(), location: None };
    assert!(is_expected(&without_file("E1880"), "E1880", &files));
    assert!(!is_expected(&without_file("E1140"), "E1140", &files));
    assert!(!is_expected(&without_file("E1880"), "E1881", &files));
    let located =
        FirstError { code: "E1140".to_owned(), location: Some("src/main.act".to_owned()) };
    assert!(is_expected(&located, "E1140", &files));
    assert!(!is_expected(&located, "E1141", &files));
    let elsewhere =
        FirstError { code: "E1140".to_owned(), location: Some("src/other.act".to_owned()) };
    assert!(!is_expected(&elsewhere, "E1140", &files));
}

#[test]
fn a_repeat_region_is_written_n_times_with_a_padded_index() {
    let body = "head\n# catalog: repeat 10\nverb step_{n}() {\n}\n# catalog: end\ntail\n";
    let text = super::repeat::expand(body).expect("a region should expand");
    assert_eq!(text.matches("verb step_").count(), 10);
    assert!(text.starts_with("head\n# catalog: repeat 10\nverb step_01() {\n}\nverb step_02()"));
    assert!(text.ends_with("verb step_10() {\n}\n# catalog: end\ntail\n"));
    let single = super::repeat::expand("# catalog: repeat 3\n{n}\n# catalog: end\n").unwrap();
    assert_eq!(single, "# catalog: repeat 3\n1\n2\n3\n# catalog: end\n");
    assert_eq!(super::repeat::expand("plain\n").unwrap(), "plain\n");
}

#[test]
fn a_malformed_repeat_region_is_an_entry_shape_error() {
    for body in [
        "# catalog: repeat 2\nx\n",
        "# catalog: repeat 2\n# catalog: repeat 2\n# catalog: end\n# catalog: end\n",
        "# catalog: repeat 0\nx\n# catalog: end\n",
        "# catalog: repeat many\nx\n# catalog: end\n",
        "x\n# catalog: end\n",
    ] {
        assert!(super::repeat::expand(body).is_err(), "{body}");
        let text =
            VALID_ENTRY.replace("verb main() {}\n```\n\n## Fix", &format!("{body}```\n\n## Fix"));
        assert!(parse_entry("E1140", &text).is_err(), "{body}");
    }
}

#[test]
fn the_llvm_marker_is_accepted_and_marks_the_group() {
    let failing = VALID_ENTRY.replace("fails=E1140", "fails=E1140,llvm");
    let entry = parse_entry("E1140", &failing).expect("the `,llvm` marker should parse");
    let group: Vec<&Block> = entry.blocks.iter().filter(|block| block.fails.is_some()).collect();
    assert!(Block::needs_llvm(&group));
    let combined = VALID_ENTRY.replace("fails=E1140", "fails=E1140,build,strict,llvm");
    assert!(parse_entry("E1140", &combined).is_ok());
    let fix = VALID_ENTRY.replace("actus compiles", "actus compiles llvm");
    let entry = parse_entry("E1140", &fix).expect("`compiles llvm` should parse");
    let group: Vec<&Block> = entry.blocks.iter().filter(|block| block.compiles).collect();
    assert!(Block::needs_llvm(&group));
    let plain = parse_entry("E1140", VALID_ENTRY).expect("entry should parse");
    assert!(!Block::needs_llvm(&plain.blocks.iter().collect::<Vec<_>>()));
}

#[test]
fn a_misplaced_llvm_marker_is_rejected() {
    for text in [
        VALID_ENTRY.replace("fails=E1140", "fails=E1140,llvm,strict"),
        VALID_ENTRY.replace("fails=E1140", "fails=E1140 llvm"),
        VALID_ENTRY.replace("actus compiles", "actus compiles,llvm"),
    ] {
        assert!(parse_entry("E1140", &text).is_err(), "{text}");
    }
    let alone = VALID_ENTRY.replace("```actus compiles\n", "```actus llvm file=src/other.act\n");
    assert!(parse_entry("E1140", &alone).is_err());
}

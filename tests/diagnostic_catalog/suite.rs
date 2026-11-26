//! The check of ADR-0123, "The catalog is checked".
//!
//! 1. every code of the registry has an entry or is on `missing.txt`, and
//!    every entry and listed code is in the registry;
//! 2. every `fails=` example reports its code first, inside the example;
//! 3. every `compiles` group passes `actus check`;
//! 4. every entry has the documented shape.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use actus::diagnostics::DiagnosticCode;

use super::entry::{Block, Entry, is_code, parse_entry};
use super::project::{TempProject, first_error, groups, inside_example};

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
    for code in &listed {
        if !registry.contains(code) {
            problems.push(format!("{code}: listed in missing.txt but not in the registry"));
        } else if entered.contains(code) {
            problems.push(format!("{code}: has an entry, remove it from missing.txt"));
        }
    }
    for code in &entered {
        if !registry.contains(code) {
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
    for entry in &entries {
        let examples: Vec<_> =
            entry.blocks.iter().filter(|block| block.section == "Example").collect();
        for group in groups(&examples) {
            let Some(failing) = group.iter().find(|block| block.fails.is_some()) else {
                continue;
            };
            let command = Block::command_for(&group);
            let project = TempProject::create(&group).expect("project should be created");
            let run = project.run(command);
            let files: Vec<&str> = group.iter().map(|block| block.target()).collect();
            match first_error(&run) {
                Some((code, location))
                    if code == entry.code && inside_example(&location, &files) => {}
                Some((code, location)) => problems.push(format!(
                    "{}: line {}: `actus {command}` reported {code} at `{location}`, expected {} in one of {files:?}\n{}",
                    entry.code, failing.line, entry.code, run.output
                )),
                None => problems.push(format!(
                    "{}: line {}: `actus {command}` reported no error\n{}",
                    entry.code, failing.line, run.output
                )),
            }
        }
    }
    assert_no_problems(&problems);
}

#[test]
fn every_compiling_block_passes_check() {
    let (entries, _) = load_entries();
    let mut problems = Vec::new();
    for entry in &entries {
        let fixes: Vec<_> = entry.blocks.iter().filter(|block| block.section == "Fix").collect();
        for group in groups(&fixes) {
            let Some(compiling) = group.iter().find(|block| block.compiles) else {
                continue;
            };
            let project = TempProject::create(&group).expect("project should be created");
            let command = Block::command_for(&group);
            let run = project.run(command);
            if !run.success {
                problems.push(format!(
                    "{}: line {}: the block does not pass `actus {command}`\n{}",
                    entry.code, compiling.line, run.output
                ));
            }
        }
    }
    assert_no_problems(&problems);
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

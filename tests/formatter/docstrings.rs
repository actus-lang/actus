//! `actus fmt` wraps documentation strings whose lines pass the 100-column
//! target width (docs/language/style-and-conventions.md).

use actus::formatter::{format_program, format_program_with_comments, source_comments};
use actus::lexer::scan;
use actus::parser::parse;

const WIDTH: usize = 100;

const SENTENCE: &str = "Close the owned Directory resource through an ins loan during best-effort early return, branch exit, and implicit scope cleanup.";
const SENTENCE_WRAPPED: [&str; 2] = [
    "Close the owned Directory resource through an ins loan during best-effort early return, branch exit,",
    "and implicit scope cleanup.",
];
const FIELD_WRAPPED: [&str; 2] = [
    "Close the owned Directory resource through an ins loan during best-effort early return, branch",
    "exit, and implicit scope cleanup.",
];

/// Formats through the AST-only path.
fn via_ast(source: &str) -> String {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    format_program(&parse(tokens).expect("source should parse"))
}

/// Formats through the comment-preserving path used by `actus fmt`.
fn via_comments(source: &str) -> String {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let comments = source_comments(source, &tokens);
    format_program_with_comments(&parse(tokens).expect("source should parse"), &comments)
}

/// Asserts both paths produce `expected` and that each is idempotent.
fn assert_both(source: &str, expected: &str) {
    for (name, format) in [("ast", via_ast as fn(&str) -> String), ("comments", via_comments)] {
        let formatted = format(source);
        assert_eq!(formatted, expected, "{name} path");
        assert_eq!(format(&formatted), formatted, "{name} path is not idempotent");
    }
}

#[test]
fn a_single_line_docstring_that_fits_is_unchanged() {
    let source = "\"\"\"Short documentation.\"\"\"\nverb f() {}\n";
    assert_both(source, source);
}

#[test]
fn a_single_line_docstring_of_exactly_100_columns_is_unchanged() {
    let text = "x".repeat(WIDTH - 6);
    let source = format!("\"\"\"{text}\"\"\"\nverb f() {{}}\n");
    assert_eq!(source.lines().next().unwrap().chars().count(), WIDTH);
    assert_both(&source, &source);
    let indented =
        format!("struct S {{\n    \"\"\"{}\"\"\"\n    a: Int,\n}}\n", "x".repeat(WIDTH - 10));
    assert_both(&indented, &indented);
}

#[test]
fn a_single_line_docstring_of_101_columns_becomes_a_block() {
    let text = "word ".repeat(18) + "wordy";
    let source = format!("\"\"\"{text}\"\"\"\nverb f() {{}}\n");
    assert_eq!(source.lines().next().unwrap().chars().count(), WIDTH + 1);
    let expected = format!("\"\"\"\n{text}\n\"\"\"\nverb f() {{}}\n");
    assert_both(&source, &expected);
}

#[test]
fn a_long_top_level_docstring_is_wrapped_into_block_form() {
    let source = format!("\"\"\"{SENTENCE}\"\"\"\nverb f() {{}}\n");
    let expected = format!(
        "\"\"\"\n{}\n{}\n\"\"\"\nverb f() {{}}\n",
        SENTENCE_WRAPPED[0], SENTENCE_WRAPPED[1]
    );
    assert_both(&source, &expected);
}

#[test]
fn a_long_indented_field_docstring_wraps_at_the_field_indentation() {
    let source = format!("struct S {{\n    \"\"\"{SENTENCE}\"\"\"\n    a: Int,\n}}\n");
    let expected = format!(
        "struct S {{\n    \"\"\"\n    {}\n    {}\n    \"\"\"\n    a: Int,\n}}\n",
        FIELD_WRAPPED[0], FIELD_WRAPPED[1]
    );
    assert_both(&source, &expected);
}

const PURPOSE: &str = "Read exactly one length-prefixed frame from the borrowed byte source, validating the declared length against the remaining capacity before any byte is copied out.";
const INPUT: &str = "source: immutable frame bytes that stay caller-owned for the whole call and are never retained by the callee after it returns to the caller.";

#[test]
fn a_contract_keeps_its_lines_and_wraps_only_the_over_long_ones() {
    let source = format!(
        "\"\"\"\ncontract:\npurpose:\n    {PURPOSE}\ninputs:\n    {INPUT}\n    flag: short entry.\n\noutputs:\n    A frame.\n\"\"\"\nverb f() {{}}\n"
    );
    let expected = "\"\"\"\ncontract:\npurpose:\n    Read exactly one length-prefixed frame from the borrowed byte source, validating the declared\n    length against the remaining capacity before any byte is copied out.\ninputs:\n    source: immutable frame bytes that stay caller-owned for the whole call and are never retained\n        by the callee after it returns to the caller.\n    flag: short entry.\n\noutputs:\n    A frame.\n\"\"\"\nverb f() {}\n";
    let formatted = via_comments(&source);
    assert_eq!(formatted, expected);
    assert_eq!(via_comments(&formatted), formatted);
    // The AST path renders contract sections itself; it still wraps and is stable.
    let ast = via_ast(&source);
    assert!(ast.lines().all(|line| line.chars().count() <= WIDTH), "{ast}");
    assert_eq!(via_ast(&ast), ast);
}

#[test]
fn short_lines_are_never_joined() {
    let source = "\"\"\"\nOne.\nTwo.\n\nThree.\n\"\"\"\nverb f() {}\n";
    assert_both(source, source);
}

#[test]
fn a_list_item_continues_two_columns_past_its_dash() {
    let item = "This item explains a long list entry that must continue under its own marker rather than under the dash itself, keeping the text aligned.";
    let source = format!("\"\"\"\nList:\n  - {item}\n  - short\n\"\"\"\nverb f() {{}}\n");
    let expected = "\"\"\"\nList:\n  - This item explains a long list entry that must continue under its own marker rather than under\n    the dash itself, keeping the text aligned.\n  - short\n\"\"\"\nverb f() {}\n";
    assert_both(&source, expected);
}

#[test]
fn a_fenced_block_is_never_changed_but_the_text_around_it_wraps() {
    let code = format!("let value = {};", "very_long_identifier ".repeat(8));
    assert!(code.chars().count() > WIDTH);
    let prose = "Prose before the example that runs well past the target width of one hundred columns, so it must wrap.";
    let source = format!("\"\"\"\n{prose}\n```act\n{code}\n```\n{prose}\n\"\"\"\nverb f() {{}}\n");
    let wrapped = "Prose before the example that runs well past the target width of one hundred columns, so it must\nwrap.";
    let expected =
        format!("\"\"\"\n{wrapped}\n```act\n{code}\n```\n{wrapped}\n\"\"\"\nverb f() {{}}\n");
    assert_both(&source, &expected);
}

#[test]
fn a_word_longer_than_the_width_stays_whole() {
    let url = format!("https://example.invalid/{}", "a".repeat(120));
    let source = format!(
        "\"\"\"\nSee {url} for the details of the protocol as they were agreed.\n\"\"\"\nverb f() {{}}\n"
    );
    let expected = format!(
        "\"\"\"\nSee\n{url}\nfor the details of the protocol as they were agreed.\n\"\"\"\nverb f() {{}}\n"
    );
    let formatted = via_comments(&source);
    assert_eq!(formatted, expected);
    assert_eq!(via_comments(&formatted), formatted);
    for line in formatted.lines() {
        assert_eq!(line, line.trim_end(), "trailing whitespace in {line:?}");
        assert!(!line.is_empty() || formatted.contains("\n\n"));
    }
}

#[test]
fn wrapped_output_has_no_trailing_whitespace_or_new_blank_lines() {
    let source = format!("\"\"\"{} \"\"\"\nverb f() {{}}\n", "wordy ".repeat(30));
    for formatted in [via_ast(&source), via_comments(&source)] {
        assert!(formatted.lines().all(|line| line == line.trim_end() && !line.is_empty()));
        assert!(formatted.lines().all(|line| line.chars().count() <= WIDTH));
    }
}

#[test]
fn hash_comments_are_not_wrapped() {
    let comment = format!("# {}", "comment ".repeat(20).trim_end());
    let source = format!("{comment}\nverb f() {{}}\n");
    assert!(comment.chars().count() > WIDTH);
    assert_eq!(via_comments(&source), source);
}

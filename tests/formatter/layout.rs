//! `actus fmt` breaks over-wide binary expression chains, keeps one space inside the braces of
//! enum struct variants and their patterns, and keeps the blank line that follows a file's
//! module documentation (docs/language/style-and-conventions.md, section 7).

use actus::formatter::{format_program, format_program_with_comments, source_comments};
use actus::lexer::scan;
use actus::parser::parse;

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

/// Wraps `body` statements in a verb so every test reads as one statement.
fn in_verb(body: &str) -> String {
    format!("verb f() -> Int {{\n{body}}}\n")
}

#[test]
fn a_long_source_chain_already_broken_over_two_lines_is_packed_to_the_width() {
    let source = in_verb(concat!(
        "    erg checks = (maximum == 255) && (minimum == -128) && (wrapped == 4) && (saturated == 255)\n",
        "        && (floor == 0) && (clamped == -128) && (flipped == -128) && (counter == 0) && (level == 255);\n",
    ));
    let expected = in_verb(concat!(
        "    erg checks = (maximum == 255) && (minimum == -128) && (wrapped == 4) && (saturated == 255)\n",
        "        && (floor == 0) && (clamped == -128) && (flipped == -128) && (counter == 0)\n",
        "        && (level == 255);\n",
    ));
    assert_both(&source, &expected);
}

#[test]
fn a_short_expression_stays_on_one_line() {
    let source = in_verb("    erg total = alpha + beta * gamma;\n");
    assert_both(&source, &source);
}

#[test]
fn a_line_of_exactly_100_columns_stays_and_101_columns_break() {
    let first = "a".repeat(41);
    let fits = in_verb(&format!("    erg x = {first} && {};\n", "b".repeat(42)));
    assert_eq!(fits.lines().nth(1).unwrap().chars().count(), 100);
    assert_both(&fits, &fits);

    let second = "b".repeat(43);
    let source = in_verb(&format!("    erg x = {first} && {second};\n"));
    let expected = in_verb(&format!("    erg x = {first}\n        && {second};\n"));
    assert_both(&source, &expected);
}

#[test]
fn assignment_and_return_break_one_level_deeper_than_their_statement() {
    let sum = "alpha_value + beta_value * alpha_value + beta_value * alpha_value + beta_value * alpha_value + 1";
    let source = in_verb(&format!(
        "    first = {sum};\n    if first {{\n        return {sum};\n    }}\n    return 0;\n"
    ));
    let expected = in_verb(concat!(
        "    first = alpha_value + beta_value * alpha_value + beta_value * alpha_value\n",
        "        + beta_value * alpha_value + 1;\n",
        "    if first {\n",
        "        return alpha_value + beta_value * alpha_value + beta_value * alpha_value\n",
        "            + beta_value * alpha_value + 1;\n",
        "    }\n",
        "    return 0;\n",
    ));
    assert_both(&source, &expected);
}

#[test]
fn an_if_condition_keeps_room_for_its_opening_brace() {
    let source = in_verb(concat!(
        "    if (alpha_value == beta_value) && (alpha_value != 1) && (beta_value != 2) && (alpha_value != 3) && (beta_value != 5) {\n",
        "        return 1;\n",
        "    }\n",
        "    return 0;\n",
    ));
    let expected = in_verb(concat!(
        "    if (alpha_value == beta_value) && (alpha_value != 1) && (beta_value != 2) && (alpha_value != 3)\n",
        "        && (beta_value != 5) {\n",
        "        return 1;\n",
        "    }\n",
        "    return 0;\n",
    ));
    assert_both(&source, &expected);
}

#[test]
fn a_case_arm_value_breaks_below_its_arm() {
    let source = in_verb(concat!(
        "    erg r = case dat message {\n",
        "        Wire.Empty => alpha_value + beta_value * alpha_value + beta_value * alpha_value + beta_value * alpha_value + 1 + beta_value * alpha_value,\n",
        "        _ => 0,\n",
        "    };\n",
        "    return r;\n",
    ));
    let expected = in_verb(concat!(
        "    erg r = case dat message {\n",
        "        Wire.Empty => alpha_value + beta_value * alpha_value + beta_value * alpha_value\n",
        "            + beta_value * alpha_value + 1 + beta_value * alpha_value,\n",
        "        _ => 0,\n",
        "    };\n",
        "    return r;\n",
    ));
    assert_both(&source, &expected);
}

#[test]
fn only_the_outermost_operator_level_breaks_and_parentheses_are_kept() {
    let source = in_verb(concat!(
        "    erg mixed = (alpha_value == 1) && (beta_value == 2) || (alpha_value == 3) && (beta_value == 4) || (alpha_value == 5) && (beta_value == 6);\n",
        "    return 0;\n",
    ));
    let expected = in_verb(concat!(
        "    erg mixed = (alpha_value == 1) && (beta_value == 2) || (alpha_value == 3) && (beta_value == 4)\n",
        "        || (alpha_value == 5) && (beta_value == 6);\n",
        "    return 0;\n",
    ));
    assert_both(&source, &expected);
}

#[test]
fn enum_struct_variants_keep_a_space_inside_their_braces() {
    let source = "enum WireMessage {\n    Byte { value: u8 },\n    Pair { code: u16, value: u32 },\n    Empty,\n}\n";
    assert_both(source, source);
}

#[test]
fn a_struct_variant_pattern_keeps_a_space_inside_its_braces() {
    let source = in_verb(concat!(
        "    erg score = case dat message {\n",
        "        WireMessage.Pair { code: code_value, value: value_value } => code_value,\n",
        "        _ => 0,\n",
        "    };\n",
        "    return score;\n",
    ));
    assert_both(&source, &source);
}

#[test]
fn a_long_struct_variant_arm_keeps_its_pattern_whole_and_wraps_the_value() {
    let source = in_verb(concat!(
        "    erg score = case dat message {\n",
        "        WireMessage.Pair { code: code_value, value: value_value } => code_value as Int + value_value as Int,\n",
        "        _ => 0,\n",
        "    };\n",
        "    return score;\n",
    ));
    let expected = in_verb(concat!(
        "    erg score = case dat message {\n",
        "        WireMessage.Pair { code: code_value, value: value_value } => code_value as Int\n",
        "            + value_value as Int,\n",
        "        _ => 0,\n",
        "    };\n",
        "    return score;\n",
    ));
    assert_both(&source, &expected);
}

#[test]
fn module_documentation_keeps_a_blank_line_before_the_next_docstring() {
    let source = concat!(
        "\"\"\"Executable coverage for the volatile effect and register layout contract.\"\"\"\n",
        "\n",
        "\"\"\"This layout declaration does not make ordinary field access volatile.\"\"\"\n",
        "meta repr(C)\n",
        "struct UartRegisters {\n",
        "    status: u32,\n",
        "}\n",
    );
    let formatted = via_comments(source);
    assert_eq!(formatted, source);
    assert_eq!(via_comments(&formatted), formatted, "not idempotent");
}

#[test]
fn module_documentation_blank_line_is_added_when_the_source_has_none() {
    let source = "\"\"\"Module.\"\"\"\n\"\"\"Item.\"\"\"\nverb f() {}\n";
    let expected = "\"\"\"Module.\"\"\"\n\n\"\"\"Item.\"\"\"\nverb f() {}\n";
    assert_eq!(via_comments(source), expected);
}

#[test]
fn a_lone_leading_docstring_stays_attached_to_its_declaration() {
    let source = "\"\"\"Item.\"\"\"\nverb f() {}\n";
    assert_eq!(via_comments(source), source);
}

#[test]
fn module_documentation_adds_no_blank_line_inside_the_next_declaration() {
    let source = concat!(
        "\"\"\"Typed failures of the module.\"\"\"\n",
        "open enum Failure {\n",
        "    \"\"\"The input is malformed.\"\"\"\n",
        "    Malformed,\n",
        "}\n",
    );
    let formatted = via_comments(source);
    assert_eq!(formatted, source);
    assert_eq!(via_comments(&formatted), formatted, "not idempotent");
}

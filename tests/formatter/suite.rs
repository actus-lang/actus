use actus::formatter::{SourceComment, format_program, format_program_with_comments};
use actus::lexer::scan;
use actus::parser::parse;

fn format_source(source: &str) -> String {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    format_program(&program)
}

#[test]
fn formats_nested_blocks_and_calls() {
    let source =
        "verb process(){erg buffer=Buffer[10];{abs view=ref buffer;inspect(view,source:view);}}";
    let expected = "verb process() {\n    erg buffer = Buffer[10];\n    {\n        abs view = ref buffer;\n        inspect(view, source: view);\n    }\n}\n";

    assert_eq!(format_source(source), expected);
}

#[test]
fn formatting_is_idempotent() {
    let source = "verb process() { erg buffer = Buffer[10]; return buffer; }";
    let formatted = format_source(source);

    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_if_else_without_collapsing_branch_boundaries() {
    let source = "verb choose(erg ready: Bool) -> Int { if ready { return 1; } else if ready { return 2; } else { return 3; } }";
    let formatted = format_source(source);

    assert!(formatted.contains("if ready {\n"));
    assert!(formatted.contains("} else if ready {\n"));
    assert!(formatted.contains("} else {\n"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_statement_if_without_a_trailing_semicolon() {
    let formatted = format_source("verb main() { if 1 == 1 { return; } }");
    assert!(formatted.contains("if 1 == 1 {\n"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn preserves_typed_float_literal_suffixes() {
    let formatted = format_source("verb main() { erg value: f32 = 1.5f32; }");
    assert!(formatted.contains("1.5f32"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_boolean_literals_without_rewriting_their_values() {
    let formatted = format_source("verb main() -> Bool { erg linked: Bool = false; return true; }");
    assert!(formatted.contains("linked: Bool = false;"));
    assert!(formatted.contains("return true;"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_typed_constants_and_keeps_their_documentation() {
    let formatted = format_source(
        "\"\"\"Frame size.\"\"\" open const FRAME_SIZE: u16 = 32u16; verb main() { return; }",
    );
    assert!(formatted.contains("\"\"\"Frame size.\"\"\"\nopen const FRAME_SIZE: u16 = 32u16;"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formatter_idempotence_holds_for_the_source_corpus() {
    let sources = [
        "verb main() -> Int { return 42; }",
        "verb process() { erg buffer = Buffer[10]; { abs view = ref buffer; inspect(view); } }",
        "verb looped() -> Int { erg value = 0; loop { value = 1; break; } return value; }",
    ];

    for source in sources {
        let formatted = format_source(source);
        assert_eq!(format_source(&formatted), formatted, "formatter changed {source:?}");
    }
}

#[test]
fn parser_round_trip_holds_for_valid_source_corpus() {
    let sources = [
        include_str!("../fixtures/parser/valid/transfer.act"),
        include_str!("../fixtures/parser/valid/drop.act"),
        include_str!("../fixtures/formatter/valid/nested_blocks.act"),
    ];

    for source in sources {
        let formatted = format_source(source);
        let reparsed = format_source(&formatted);
        assert_eq!(reparsed, formatted, "formatted source did not round-trip");
    }
}

#[test]
fn matches_nested_block_formatter_snapshot() {
    let source =
        "verb process(){erg buffer=Buffer[10];{abs view=ref buffer;inspect(view,source:view);}}";
    let actual = format_source(source);
    let expected = include_str!("../fixtures/formatter/snapshots/nested_blocks.format.snap");

    assert_eq!(actual, expected);
}

#[test]
fn preserves_docstrings_and_module_declarations() {
    let source = "\"\"\"Keep this module documentation.\"\"\"\nimport io;\nopen stdout;\n\"\"\"Keep this verb documentation.\"\"\"\nverb main() -> Int { return 0; }\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let comments = tokens
        .iter()
        .filter(|token| matches!(token.kind, actus::lexer::TokenKind::DocString(_)))
        .map(|token| SourceComment {
            span: token.span,
            text: source[token.span.start..token.span.end].to_owned(),
        })
        .collect::<Vec<_>>();
    let program = parse(tokens).expect("source should parse");
    let formatted = format_program_with_comments(&program, &comments);

    assert!(formatted.contains("\"\"\"Keep this module documentation.\"\"\""));
    assert!(formatted.contains("\"\"\"Keep this verb documentation.\"\"\""));
    assert!(formatted.contains("import io;"));
    assert!(formatted.contains("open stdout;"));
}

#[test]
fn preserves_docstrings_before_declarations() {
    let source = "\"\"\"Keeps this public documentation.\"\"\"\nopen struct Reader { erg buffer: Buffer, }\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let comments = tokens
        .iter()
        .filter(|token| matches!(token.kind, actus::lexer::TokenKind::DocString(_)))
        .map(|token| SourceComment {
            span: token.span,
            text: source[token.span.start..token.span.end].to_owned(),
        })
        .collect::<Vec<_>>();
    let program = parse(tokens).expect("source should parse");
    let formatted = format_program_with_comments(&program, &comments);

    assert!(formatted.contains("\"\"\"Keeps this public documentation.\"\"\""));
    assert!(formatted.contains("open struct Reader"));
}

#[test]
fn preserves_roles_in_wrapped_external_parameters() {
    let source = r#"unsafe extern "C" verb append_range(
        ins target: Buffer,
        abs source: Buffer,
        erg offset: Int,
        erg length: Int
    ) -> Int;"#;
    let formatted = format_source(source);

    assert!(formatted.contains("ins target: Buffer"));
    assert!(formatted.contains("abs source: Buffer"));
    assert!(formatted.contains("erg offset: Int"));
    assert!(formatted.contains("erg length: Int"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn keeps_spaces_in_generic_call_types() {
    let source = "verb main() -> Result[Int, IoError] { return Result[Int, IoError].Ok(1); }";
    let formatted = format_source(source);

    assert!(formatted.contains("Result[Int, IoError].Ok(1)"), "formatted: {formatted}");
    assert_eq!(format_source(&formatted), formatted);
}

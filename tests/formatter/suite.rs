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
fn preserves_comments_and_module_declarations() {
    let source = "// keep this comment\nimport io;\nopen stdout;\nverb main() -> Int {\n    // keep this body comment\n    return 0;\n}\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let comments = tokens
        .iter()
        .filter(|token| {
            matches!(
                token.kind,
                actus::lexer::TokenKind::Comment | actus::lexer::TokenKind::DocString(_)
            )
        })
        .map(|token| SourceComment {
            span: token.span,
            text: source[token.span.start..token.span.end].to_owned(),
        })
        .collect::<Vec<_>>();
    let program = parse(tokens).expect("source should parse");
    let formatted = format_program_with_comments(&program, &comments);

    assert!(formatted.contains("// keep this comment"));
    assert!(formatted.contains("// keep this body comment"));
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
fn keeps_trailing_comments_inside_their_function_block() {
    let source = "verb main() -> Int {\n    // stays inside main\n    return 0;\n    // stays before the closing brace\n}\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let comments = tokens
        .iter()
        .filter(|token| matches!(token.kind, actus::lexer::TokenKind::Comment))
        .map(|token| SourceComment {
            span: token.span,
            text: source[token.span.start..token.span.end].to_owned(),
        })
        .collect::<Vec<_>>();
    let program = parse(tokens).expect("source should parse");
    let formatted = format_program_with_comments(&program, &comments);

    let closing_brace = formatted.rfind('}').expect("formatted block should close");
    let trailing_comment = formatted.find("stays before").expect("trailing comment should remain");
    assert!(trailing_comment < closing_brace);
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

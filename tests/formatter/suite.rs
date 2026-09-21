use actus::formatter::format_program;
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
        "verb process(){erg buffer=allocate(10);{abs view=ref buffer;inspect(view,source:view);}}";
    let expected = "verb process() {\n    erg buffer = allocate(10);\n    {\n        abs view = ref buffer;\n        inspect(view, source: view);\n    }\n}\n";

    assert_eq!(format_source(source), expected);
}

#[test]
fn formatting_is_idempotent() {
    let source = "verb process() { erg buffer = allocate(10); return buffer; }";
    let formatted = format_source(source);

    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formatter_idempotence_holds_for_the_source_corpus() {
    let sources = [
        "verb main() -> Int { return 42; }",
        "verb process() { erg buffer = allocate(10); { abs view = ref buffer; inspect(view); } }",
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
        "verb process(){erg buffer=allocate(10);{abs view=ref buffer;inspect(view,source:view);}}";
    let actual = format_source(source);
    let expected = include_str!("../fixtures/formatter/snapshots/nested_blocks.format.snap");

    assert_eq!(actual, expected);
}

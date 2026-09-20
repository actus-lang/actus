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

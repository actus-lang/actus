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
fn formats_bounded_for_ranges_idempotently() {
    let formatted = format_source(
        "verb main(){for erg index:u32 in 0u32..4u32 { if index == 2u32 { break; } }}",
    );
    assert!(formatted.contains("for erg index: u32 in 0u32 .. 4u32"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn canonicalizes_repeat_to_bounded_for_idempotently() {
    let formatted =
        format_source("verb main(){repeat erg index:u32 in 0u32..4u32 { total += index; }}");
    assert!(formatted.contains("for erg index: u32 in 0u32 .. 4u32"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_named_pack_offsets_idempotently() {
    let formatted = format_source(
        "const OFFSET:u16=8u16;pack Frame{erg storage:Array[u8,2];layout little;fields{erg marker:u8 at OFFSET;}}",
    );
    assert!(formatted.contains("marker: u8 at OFFSET;"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_indexed_pack_fields_idempotently() {
    let formatted = format_source(
        "pack Example{erg storage:Array[u8,32];layout little;fields{erg links:Array[u32,8] at 192;}}",
    );
    assert!(formatted.contains("links: Array[u32, 8] at 192;"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_serialization_contracts_idempotently() {
    let formatted = format_source(
        "serialize Frame from FramePack{layout little;version u16 at 0;payload bytes at 2 length 16;checksum crc32 over 0..18 at 18;}",
    );
    assert!(formatted.contains("payload bytes at 2 length 16;"));
    assert!(formatted.contains("checksum crc32 over 0 .. 18 at 18;"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_scalar_reuse_and_indexed_selection_idempotently() {
    let formatted = format_source(
        "verb main(){erg values:Array[u32,2]=Array[u32,2]();erg value:u32=copy(value:abs values[1]);}",
    );
    assert!(formatted.contains("copy(value: abs values[1])"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn preserves_omitted_inferred_roles_without_inventing_source_text() {
    let formatted = format_source(
        "verb read(abs value: Int) -> Int { return 0; } verb main() -> Int { erg source = 1; abs view = ref source; return read(value: view); }",
    );
    assert!(formatted.contains("read(value: view)"));
    assert!(!formatted.contains("read(value: abs view)"));
    assert_eq!(format_source(&formatted), formatted);
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
fn formats_the_complete_monotonic_time_surface_idempotently() {
    let source = concat!(
        "import std::time;\n",
        "verb main() -> Int {\n",
        "erg started: Instant = now();\n",
        "erg budget: Duration = duration_nanos(nanos: erg 1000u64);\n",
        "erg deadline = deadline_after(start: abs started, duration: abs budget);\n",
        "erg timer = timer_one_shot();\n",
        "erg delayed = delay(duration: abs budget);\n",
        "return 0;\n",
        "}\n",
    );
    let formatted = format_source(source);

    assert!(formatted.contains("import std::time;"));
    assert!(formatted.contains("deadline_after(start: abs started"));
    assert!(formatted.contains("timer_one_shot()"));
    assert!(formatted.contains("delay(duration: abs budget)"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_the_public_utf8_boundary_idempotently() {
    let source = concat!(
        "import std::string;\n",
        "verb copy_text(dat storage: Buffer, abs source: String) -> Result[Utf8Buffer, StringError] {\n",
        "erg result = utf8_from_string(text: abs source, storage: dat storage);\n",
        "return result;\n",
        "}\n",
    );
    let formatted = format_source(source);

    assert!(formatted.contains("import std::string;"));
    assert!(formatted.contains("utf8_from_string(text: abs source, storage: dat storage)"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_the_public_wire_checksum_surface_idempotently() {
    let source = concat!(
        "import std::wire;\n",
        "verb checksum() -> Int {\n",
        "erg input: Buffer = Buffer[0];\n",
        "erg start: u32 = 0u32;\n",
        "erg end: u32 = 0u32;\n",
        "erg result = wire_crc16_ccitt(input: abs input, start: erg start, end: erg end);\n",
        "return 0;\n",
        "}\n",
    );
    let formatted = format_source(source);

    assert!(formatted.contains("import std::wire;"));
    assert!(formatted.contains("wire_crc16_ccitt("));
    assert!(formatted.contains("start:"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_const_generic_declarations_without_changing_meaning() {
    let formatted = format_source(
        "struct Fabric[N: Usize] { cells: Array[Int, N], } verb main() -> Bool { return false; }",
    );
    assert!(formatted.contains("struct Fabric[N: Usize] {"));
    assert!(formatted.contains("cells: Array[Int, N]"));
    assert!(formatted.contains("return false;"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_array_backed_packs_idempotently_with_layout_metadata() {
    let source = "pack Frame { erg storage: Array[u8, 4]; layout big; fields { erg marker: u8 at 0; abs tail: u8 at 24 = 0; } }";
    let formatted = format_source(source);

    assert!(formatted.contains("erg storage: Array[u8, 4];"));
    assert!(formatted.contains("layout big;"));
    assert!(formatted.contains("erg marker: u8 at 0;"));
    assert!(formatted.contains("abs tail: u8 at 24 = 0;"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_pack_array_element_types_idempotently() {
    let source = "pack Cell { erg storage: u8; layout little; fields { erg marker: u8 at 0; } } struct Fabric { cells: Array[Cell, 2], }";
    let formatted = format_source(source);

    assert!(formatted.contains("cells: Array[Cell, 2]"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn keeps_pack_storage_and_field_documentation_in_place() {
    let source = concat!(
        "\"\"\"Frame layout.\"\"\"\n",
        "pack Frame {\n",
        "\"\"\"Raw storage bytes.\"\"\"\n",
        "erg storage: Array[u8, 2];\n",
        "layout little; fields {\n",
        "\"\"\"Message marker.\"\"\"\n",
        "erg marker: u8 at 0;\n",
        "abs tail: u8 at 8 = 0;\n",
        "}\n}\n",
    );
    let formatted = format_source(source);

    assert!(formatted.contains("\"\"\"Raw storage bytes.\"\"\"\n    erg storage"));
    assert!(formatted.contains("\"\"\"Message marker.\"\"\"\n        erg marker"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_nested_places_without_collapsing_selectors() {
    let formatted = format_source(
        "verb main() { erg values: Array[Int, 2] = Array[Int, 2](); values[1].field = 7; values[1].field += 1; }",
    );
    assert!(formatted.contains("values[1].field = 7;"));
    assert!(formatted.contains("values[1].field += 1;"));
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
fn round_trips_structured_verb_contracts() {
    let source = r###"
"""
contract:
ownership:
    The caller keeps ownership.
purpose:
    Read one frame.
"""
verb read_frame(abs source: Buffer) -> Int { return 0; }
"###;
    let formatted = format_source(source);
    assert!(formatted.contains("contract:"));
    assert!(formatted.contains("purpose:"));
    assert!(formatted.contains("The caller keeps ownership."));
    assert!(formatted.find("purpose:") < formatted.find("ownership:"));
    assert_eq!(format_source(&formatted), formatted);
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

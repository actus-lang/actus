use actus::lexer::{LexErrorKind, SourceSpan, TokenKind, scan};

#[test]
fn scans_keywords_and_punctuation() {
    let (tokens, errors) = scan("verb process(erg value: Buffer) {");

    assert!(errors.is_empty());
    assert_eq!(
        tokens.iter().map(|token| &token.kind).collect::<Vec<_>>(),
        vec![
            &TokenKind::Verb,
            &TokenKind::Identifier("process".to_owned()),
            &TokenKind::LeftParen,
            &TokenKind::Erg,
            &TokenKind::Identifier("value".to_owned()),
            &TokenKind::Colon,
            &TokenKind::Identifier("Buffer".to_owned()),
            &TokenKind::RightParen,
            &TokenKind::LeftBrace,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn scans_the_instrumental_role_keyword() {
    let (tokens, errors) = scan("verb update(ins buffer: Buffer) { }");

    assert!(errors.is_empty());
    assert_eq!(tokens[3].kind, TokenKind::Ins);
}

#[test]
fn scans_width_qualified_integer_types_and_hex_literals() {
    let (tokens, errors) = scan("u1 u8 u32 u128 i16 f32 f64 Void 0xDEADBEEF 0x0");

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens[0].kind, TokenKind::IntType { signed: false, width: 1 });
    assert_eq!(tokens[1].kind, TokenKind::IntType { signed: false, width: 8 });
    assert_eq!(tokens[2].kind, TokenKind::IntType { signed: false, width: 32 });
    assert_eq!(tokens[3].kind, TokenKind::IntType { signed: false, width: 128 });
    assert_eq!(tokens[4].kind, TokenKind::IntType { signed: true, width: 16 });
    assert_eq!(tokens[5].kind, TokenKind::FloatType { width: 32 });
    assert_eq!(tokens[6].kind, TokenKind::FloatType { width: 64 });
    assert_eq!(tokens[7].kind, TokenKind::VoidType);
    assert_eq!(tokens[8].kind, TokenKind::Integer("0xDEADBEEF".to_owned()));
    assert_eq!(tokens[9].kind, TokenKind::Integer("0x0".to_owned()));
}

#[test]
fn rejects_invalid_width_types_and_hex_literals() {
    let (_, errors) = scan("u0 i0 u129 i256 0x 0x12G");

    assert_eq!(errors.len(), 6);
    assert!(
        errors
            .iter()
            .take(4)
            .all(|error| { matches!(error.kind, LexErrorKind::InvalidIntegerType(_)) })
    );
    assert!(matches!(errors[4].kind, LexErrorKind::InvalidHexLiteral(_)));
    assert!(matches!(errors[5].kind, LexErrorKind::InvalidHexLiteral(_)));
}

#[test]
fn scans_generic_delimiters() {
    let (tokens, errors) = scan("Option[Int, Result[Bool, String]]");

    assert!(errors.is_empty());
    assert_eq!(
        tokens.iter().map(|token| &token.kind).collect::<Vec<_>>(),
        vec![
            &TokenKind::Identifier("Option".to_owned()),
            &TokenKind::LeftBracket,
            &TokenKind::Identifier("Int".to_owned()),
            &TokenKind::Comma,
            &TokenKind::Identifier("Result".to_owned()),
            &TokenKind::LeftBracket,
            &TokenKind::Identifier("Bool".to_owned()),
            &TokenKind::Comma,
            &TokenKind::Identifier("String".to_owned()),
            &TokenKind::RightBracket,
            &TokenKind::RightBracket,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn scans_enum_and_case_keywords() {
    let (tokens, errors) = scan("enum Color { Red, } case true => _");

    assert!(errors.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::Enum);
    assert_eq!(tokens[6].kind, TokenKind::Case);
    assert!(tokens.iter().any(|token| token.kind == TokenKind::True));
    assert!(tokens.iter().any(|token| token.kind == TokenKind::FatArrow));
    assert!(tokens.iter().any(|token| token.kind == TokenKind::Underscore));
}

#[test]
fn scans_dynamic_as_a_reserved_keyword() {
    let (tokens, errors) = scan("abs writer: dynamic Writer");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens[0].kind, TokenKind::Abs);
    assert_eq!(tokens[3].kind, TokenKind::Dynamic);
    assert_eq!(tokens[4].kind, TokenKind::Identifier("Writer".to_owned()));
}

#[test]
fn scans_struct_fields_and_dot_access() {
    let (tokens, errors) = scan("struct Point { erg payload: Buffer, x: F32 } point.x");

    assert!(errors.is_empty());
    assert_eq!(
        tokens.iter().map(|token| &token.kind).collect::<Vec<_>>(),
        vec![
            &TokenKind::Struct,
            &TokenKind::Identifier("Point".to_owned()),
            &TokenKind::LeftBrace,
            &TokenKind::Erg,
            &TokenKind::Identifier("payload".to_owned()),
            &TokenKind::Colon,
            &TokenKind::Identifier("Buffer".to_owned()),
            &TokenKind::Comma,
            &TokenKind::Identifier("x".to_owned()),
            &TokenKind::Colon,
            &TokenKind::Identifier("F32".to_owned()),
            &TokenKind::RightBrace,
            &TokenKind::Identifier("point".to_owned()),
            &TokenKind::Dot,
            &TokenKind::Identifier("x".to_owned()),
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn scans_comparison_and_logical_operators() {
    let (tokens, errors) = scan("< <= > >= == != ! && || % & | ^ ~ << >> => -> =");

    assert!(errors.is_empty());
    assert_eq!(
        tokens.iter().map(|token| &token.kind).collect::<Vec<_>>(),
        vec![
            &TokenKind::LessThan,
            &TokenKind::LessEquals,
            &TokenKind::GreaterThan,
            &TokenKind::GreaterEquals,
            &TokenKind::DoubleEquals,
            &TokenKind::BangEquals,
            &TokenKind::Bang,
            &TokenKind::AndAnd,
            &TokenKind::OrOr,
            &TokenKind::Percent,
            &TokenKind::Ampersand,
            &TokenKind::Pipe,
            &TokenKind::Caret,
            &TokenKind::Tilde,
            &TokenKind::ShiftLeft,
            &TokenKind::ShiftRight,
            &TokenKind::FatArrow,
            &TokenKind::Arrow,
            &TokenKind::Equals,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn records_half_open_byte_spans() {
    let (tokens, errors) = scan("erg buf");

    assert!(errors.is_empty());
    assert_eq!(tokens[0].span, SourceSpan::new(0, 3));
    assert_eq!(tokens[1].span, SourceSpan::new(4, 7));
    assert_eq!(tokens[2].span, SourceSpan::new(7, 7));
}

#[test]
fn skips_comments_and_scans_literals() {
    let (tokens, errors) = scan("// ignored\n42 \"hello\\\"\"");

    assert!(errors.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::Integer("42".to_owned()));
    assert_eq!(tokens[1].kind, TokenKind::StringLiteral("hello\\\"".to_owned()));
}

#[test]
fn collects_unexpected_character_errors() {
    let (tokens, errors) = scan("erg @ buf");

    assert_eq!(tokens.len(), 3);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnexpectedCharacter('@'));
    assert_eq!(errors[0].span, SourceSpan::new(4, 5));
}

#[test]
fn rejects_unterminated_strings() {
    let (tokens, errors) = scan("\"unterminated");

    assert_eq!(tokens.len(), 1);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnterminatedString);
    assert_eq!(errors[0].span, SourceSpan::new(0, 13));
}

#[test]
fn scans_docstrings_and_strips_common_markdown_indentation() {
    let source = "\"\"\"\n        Summary.\n\n        More detail.\n        \"\"\"";
    let (tokens, errors) = scan(source);

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens[0].kind, TokenKind::DocString("Summary.\n\nMore detail.".to_owned()));
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, source.len());
}

#[test]
fn rejects_unterminated_docstrings() {
    let (tokens, errors) = scan("\"\"\"missing terminator");

    assert_eq!(tokens.len(), 1);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnterminatedDocString);
}

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
    let (tokens, errors) = scan("< <= > >= == != ! => -> =");

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

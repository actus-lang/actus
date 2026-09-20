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

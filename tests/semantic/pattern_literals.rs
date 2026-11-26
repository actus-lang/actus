//! Bug #41: an integer literal pattern needs an integer subject that can hold it.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

fn analysis(source: &str) -> Result<(), actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map(|_| ())
}

fn matching(prelude: &str, subject: &str, pattern: &str) -> String {
    format!(
        "{prelude}\nverb name(abs value: {subject}) -> Int {{ return case value {{ {pattern} => 1, _ => 0, }}; }}\nverb main() {{ }}"
    )
}

#[test]
fn integer_literal_is_rejected_on_non_integer_subjects() {
    let cases = [
        ("", "Bool", "1"),
        ("", "f32", "1"),
        ("", "f64", "-1"),
        ("enum Color { Red, Green, }", "Color", "1"),
        ("struct Point { x: Int, }", "Point", "1"),
    ];
    for (prelude, subject, pattern) in cases {
        let source = matching(prelude, subject, pattern);
        let error = analysis(&source).expect_err(&source);
        let diagnostic = actus::diagnostics::semantic_diagnostic(&error);
        assert_eq!(diagnostic.code(), "E1048", "{source}");
        let span = diagnostic.span();
        assert_eq!(&source[span.start..span.end], pattern, "{source}");
    }
}

#[test]
fn integer_subjects_accept_literals_that_fit() {
    let cases = [
        ("Int", "-1"),
        ("Int", "7"),
        ("i8", "-128"),
        ("i8", "127"),
        ("u8", "255"),
        ("u7", "127"),
        ("i3", "-4"),
        ("i3", "3"),
        ("u16", "0x10"),
        ("i64", "-5"),
        ("Usize", "5"),
    ];
    for (subject, pattern) in cases {
        let source = matching("", subject, pattern);
        analysis(&source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
}

#[test]
fn integer_subjects_reject_literals_that_do_not_fit() {
    for (subject, pattern) in
        [("u8", "-1"), ("i8", "128"), ("i3", "4"), ("u7", "128"), ("Usize", "-1")]
    {
        let source = matching("", subject, pattern);
        let error = analysis(&source).expect_err(&source);
        let diagnostic = actus::diagnostics::semantic_diagnostic(&error);
        assert_eq!(diagnostic.code(), "E1048", "{source}");
    }
}

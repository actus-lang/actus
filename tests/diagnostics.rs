use actus::diagnostics::{
    Diagnostic, DiagnosticPhase, DiagnosticSeverity, lex_diagnostic, render_colored_diagnostic,
    render_diagnostic, render_json_diagnostics, render_semantic_error, semantic_diagnostic,
    sort_diagnostics,
};
use actus::lexer::{SourceSpan, scan};
use actus::parser::parse;
use actus::semantic::analyze;

#[test]
fn renders_stable_semantic_error_codes_and_locations() {
    let source = "verb main() {\n    return missing;\n}\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("undeclared binding should fail");

    assert_eq!(
        render_semantic_error(source, &error),
        "error[E1003] at 2:12: undeclared identifier `missing`"
    );
    let diagnostic = semantic_diagnostic(&error);
    assert_eq!(diagnostic.code(), "E1003");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
}

#[test]
fn diagnostic_model_is_independent_from_terminal_rendering() {
    let source = "ok\nvalue\n";
    let diagnostic = Diagnostic::warning("E1800", SourceSpan::new(3, 8), "strict warning")
        .with_source_path("src/main.act")
        .with_explanation("strict mode rejects unresolved warnings")
        .with_suggestion("resolve the warning before building");

    assert_eq!(diagnostic.code(), "E1800");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Warning);
    assert_eq!(diagnostic.span(), SourceSpan::new(3, 8));
    assert_eq!(diagnostic.source_path(), Some("src/main.act"));
    assert_eq!(diagnostic.message(), "strict warning");
    assert_eq!(diagnostic.explanation(), Some("strict mode rejects unresolved warnings"));
    assert_eq!(diagnostic.suggestion(), Some("resolve the warning before building"));
    assert_eq!(render_diagnostic(source, &diagnostic), "warning[E1800] at 2:1: strict warning");
}

#[test]
fn diagnostics_sort_by_source_location_phase_and_code() {
    let mut diagnostics = vec![
        Diagnostic::error("E2000", SourceSpan::new(4, 5), "later")
            .with_phase(DiagnosticPhase::Semantic)
            .with_source_path("src/b.act"),
        Diagnostic::error("E1000", SourceSpan::new(0, 1), "first")
            .with_phase(DiagnosticPhase::Parser)
            .with_source_path("src/a.act"),
        Diagnostic::error("E0001", SourceSpan::new(0, 1), "lexical")
            .with_phase(DiagnosticPhase::Lexical)
            .with_source_path("src/a.act"),
    ];

    sort_diagnostics(&mut diagnostics);

    assert_eq!(diagnostics[0].code(), "E0001");
    assert_eq!(diagnostics[1].code(), "E1000");
    assert_eq!(diagnostics[2].code(), "E2000");
}

#[test]
fn json_renderer_preserves_shared_diagnostic_metadata() {
    let diagnostics = vec![
        Diagnostic::warning("E1800", SourceSpan::new(3, 8), "strict warning")
            .with_phase(DiagnosticPhase::Configuration)
            .with_source_path("Actus.toml")
            .with_explanation("strict mode rejects unresolved warnings")
            .with_suggestion("resolve the warning before building"),
        Diagnostic::error("E0001", SourceSpan::new(0, 1), "unexpected character")
            .with_phase(DiagnosticPhase::Lexical)
            .with_source_path("src/main.act"),
    ];

    let encoded = render_json_diagnostics(&diagnostics).expect("diagnostics should serialize");
    let entries: serde_json::Value = serde_json::from_str(&encoded).expect("valid JSON output");

    assert_eq!(entries[0]["code"], "E1800");
    assert_eq!(entries[0]["phase"], "configuration");
    assert_eq!(entries[0]["severity"], "warning");
    assert_eq!(entries[0]["sourcePath"], "Actus.toml");
    assert_eq!(entries[0]["span"]["start"], 3);
    assert_eq!(entries[0]["explanation"], "strict mode rejects unresolved warnings");
    assert_eq!(entries[1]["code"], "E0001");
    assert_eq!(entries[1]["phase"], "lexical");
    assert_eq!(diagnostics[0].code(), "E1800");
}

#[test]
fn colored_renderer_preserves_plain_output_when_disabled() {
    let diagnostic = Diagnostic::error("E1003", SourceSpan::new(0, 1), "undeclared name")
        .with_phase(DiagnosticPhase::Semantic);
    let plain = render_diagnostic("x", &diagnostic);

    assert_eq!(render_colored_diagnostic("x", &diagnostic, false), plain);
    assert_eq!(
        render_colored_diagnostic("x", &diagnostic, true),
        "\x1b[31merror[E1003]\x1b[0m at 1:1: undeclared name"
    );
}

#[test]
fn lexical_diagnostics_have_stable_model_fields() {
    let (_, errors) = scan("@");
    let diagnostic = lex_diagnostic(&errors[0]);

    assert_eq!(diagnostic.code(), "E0001");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
    assert_eq!(diagnostic.span(), SourceSpan::new(0, 1));
    assert_eq!(
        render_diagnostic("@", &diagnostic),
        "error[E0001] at 1:1: unexpected character `@`"
    );
}

#[test]
fn matches_semantic_diagnostic_snapshot() {
    let source = include_str!("fixtures/semantic/invalid/use_after_drop.act");
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("fixture should parse");
    let error = analyze(&program).expect_err("use after drop should fail");
    let actual = render_semantic_error(source, &error);
    let expected = include_str!("fixtures/semantic/snapshots/use_after_drop.diag.snap");

    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn renders_case_semantic_diagnostic_codes() {
    let sources = [
        (
            "enum Color { Red, Green, } verb main(erg color: Color) -> Int { return case color { Color.Red => 1, }; }",
            "E1045",
        ),
        (
            "enum Color { Red, } verb main(erg color: Color) -> Int { return case color { _ => 0, Color.Red => 1, }; }",
            "E1046",
        ),
        (
            "enum Color { Red, Green, } verb main(erg color: Color) -> Int { return case color { Color.Red => 1, Color.Red => 2, _ => 0, }; }",
            "E1047",
        ),
    ];
    for (source, code) in sources {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let program = parse(tokens).expect("source should parse");
        let error = analyze(&program).expect_err("case semantic validation should fail");
        assert!(render_semantic_error(source, &error).contains(&format!("error[{code}]")));
    }
}

#[test]
fn renders_exclusive_loan_alias_diagnostic() {
    let source = "verb merge(ins left: Buffer, abs view: Buffer) { } verb main() { erg buffer = Buffer[1]; merge(left: ins buffer, view: abs buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("ins and abs aliasing should fail");
    assert_eq!(
        render_semantic_error(source, &error),
        "error[E1065] at 1:124: exclusive loan aliases resource `buffer` more than once"
    );
}

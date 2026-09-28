use actus::diagnostics::{
    DiagnosticSeverity, render_colored_diagnostic, render_diagnostic, render_json_diagnostics,
    semantic_diagnostic,
};
use actus::lexer::{SourceSpan, scan};
use actus::lsp::analyze_document;
use actus::parser::parse;
use actus::semantic::analyze;

#[test]
fn all_diagnostic_adapters_preserve_one_fixture() {
    let source = "verb main() -> Int { return missing; }\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("fixture should parse");
    let error = analyze(&program).expect_err("fixture should fail semantic analysis");
    let diagnostic = semantic_diagnostic(&error);

    assert_eq!(diagnostic.code(), "E1003");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
    assert_eq!(diagnostic.span(), SourceSpan::new(28, 35));
    assert_eq!(
        render_diagnostic(source, &diagnostic),
        "error[E1003] at 1:29: undeclared identifier `missing`"
    );
    assert_eq!(
        render_colored_diagnostic(source, &diagnostic, true),
        "\x1b[31merror[E1003]\x1b[0m at 1:29: undeclared identifier `missing`"
    );
    assert_json_metadata(&diagnostic);
    assert_lsp_metadata(source);
}

fn assert_json_metadata(diagnostic: &actus::diagnostics::Diagnostic) {
    let encoded = render_json_diagnostics(std::slice::from_ref(diagnostic)).expect("serialize");
    let json: serde_json::Value = serde_json::from_str(&encoded).expect("valid JSON output");
    assert_eq!(json[0]["code"], "E1003");
    assert_eq!(json[0]["severity"], "error");
    assert_eq!(json[0]["span"]["start"], 28);
    assert_eq!(json[0]["span"]["end"], 35);
}

fn assert_lsp_metadata(source: &str) {
    let lsp =
        analyze_document("file:///tmp/actus-diagnostic-parity.act", source, &Default::default());
    assert_eq!(lsp.len(), 1);
    assert_eq!(lsp[0].code.as_deref(), Some("E1003"));
    assert_eq!(lsp[0].severity, 1);
    assert_eq!(lsp[0].range.start.character, 28);
    assert_eq!(lsp[0].range.end.character, 35);
}

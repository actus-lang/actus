use actus::diagnostics::{
    Diagnostic, DiagnosticPhase, render_colored_diagnostic, render_diagnostic,
};
use actus::lexer::SourceSpan;

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

use std::process::{Command, Output};

use actus::diagnostics::{
    Diagnostic, DiagnosticCatalogError, DiagnosticDefinition, DiagnosticPhase, DiagnosticSeverity,
    StrictDiagnosticCategory, lex_diagnostic, render_colored_diagnostic, render_diagnostic,
    render_json_diagnostics, render_lex_error, render_parse_error, render_semantic_error,
    semantic_diagnostic, sort_diagnostics, strict_code_category, validate_diagnostic_catalog,
};
use actus::lexer::{SourceSpan, scan};
use actus::lsp::analyze_document;
use actus::modules::{ModuleError, ModuleResolutionError};
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

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
fn renders_unknown_verb_diagnostics_with_a_stable_code() {
    let source = "verb main() { missing_verb(1); }\n";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("unknown verbs must fail semantic analysis");

    assert_eq!(
        render_semantic_error(source, &error),
        "error[E1069] at 1:15: unknown verb `missing_verb`"
    );
    assert_eq!(semantic_diagnostic(&error).code(), "E1069");
}

#[test]
fn renders_malformed_type_name_diagnostics_with_a_stable_code() {
    let error = actus::semantic::SemanticError {
        kind: SemanticErrorKind::MalformedTypeName { name: "Box[".to_owned() },
        span: SourceSpan::new(0, 4),
    };

    assert_eq!(
        render_semantic_error("Box[", &error),
        "error[E1080] at 1:1: malformed type name `Box[`"
    );
    assert_eq!(semantic_diagnostic(&error).code(), "E1080");
}

#[test]
fn renders_duplicate_pack_diagnostics_with_a_stable_code() {
    let error = actus::semantic::SemanticError {
        kind: SemanticErrorKind::DuplicatePackName { name: "Register".to_owned() },
        span: SourceSpan::new(0, 8),
    };

    assert_eq!(
        render_semantic_error("pack Register", &error),
        "error[E1081] at 1:1: duplicate pack declaration `Register`"
    );
    assert_eq!(semantic_diagnostic(&error).code(), "E1081");
}

#[test]
fn renders_unknown_metadata_with_a_stable_code() {
    let source = "meta experimental";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let error = parse(tokens).expect_err("unknown metadata should fail parsing");

    assert_eq!(
        render_parse_error(source, &error),
        "error[E0006] at 1:6: unknown metadata attribute `experimental`"
    );
}

#[test]
fn renders_unknown_keyword_with_a_stable_parser_code() {
    let (tokens, errors) = scan("verbb main() { return 0; }");
    assert!(errors.is_empty());
    let error = parse(tokens).expect_err("unknown declaration keyword should fail");

    assert_eq!(
        render_parse_error("verbb main() { return 0; }", &error),
        "error[E0009] at 1:1: unknown keyword `verbb`"
    );
}

#[test]
fn renders_missing_facade_with_a_stable_module_code() {
    let error = ModuleError::Resolution(ModuleResolutionError::MissingFacade {
        module: "std::missing".to_owned(),
        expected: std::path::PathBuf::from("library/std/src/missing/missing.act"),
    });

    let diagnostic = actus::diagnostics::module_diagnostic(&error);
    assert_eq!(diagnostic.code(), "E1101");
    assert_eq!(diagnostic.phase(), DiagnosticPhase::Module);
    assert_eq!(
        render_diagnostic("", &diagnostic),
        "error[E1101] at 1:1: module `std::missing` is missing its canonical facade `library/std/src/missing/missing.act`"
    );
}

#[test]
fn renders_unknown_facade_sibling_with_a_stable_module_code() {
    let error = ModuleError::UnknownSiblingModule {
        module: "std::io".to_owned(),
        sibling: "missing".to_owned(),
        facade: std::path::PathBuf::from("library/std/src/io/io.act"),
    };

    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1104");
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

    let encoded = render_json_diagnostics(std::slice::from_ref(&diagnostic))
        .expect("fixture should serialize");
    let json: serde_json::Value = serde_json::from_str(&encoded).expect("valid diagnostic JSON");
    assert_eq!(json[0]["code"], "E1003");
    assert_eq!(json[0]["severity"], "error");
    assert_eq!(json[0]["span"]["start"], 28);
    assert_eq!(json[0]["span"]["end"], 35);

    let lsp =
        analyze_document("file:///tmp/actus-diagnostic-parity.act", source, &Default::default());
    assert_eq!(lsp.len(), 1);
    assert_eq!(lsp[0].code.as_deref(), Some("E1003"));
    assert_eq!(lsp[0].severity, 1);
    assert_eq!(lsp[0].range.start.line, 0);
    assert_eq!(lsp[0].range.start.character, 28);
    assert_eq!(lsp[0].range.end.line, 0);
    assert_eq!(lsp[0].range.end.character, 35);
}

#[test]
fn renderer_choice_preserves_order_validation_and_exit_status() {
    let diagnostics = vec![
        Diagnostic::error("E1003", SourceSpan::new(4, 5), "later")
            .with_phase(DiagnosticPhase::Semantic)
            .with_source_path("src/b.act"),
        Diagnostic::error("E0001", SourceSpan::new(0, 1), "first")
            .with_phase(DiagnosticPhase::Lexical)
            .with_source_path("src/a.act"),
    ];
    let original = diagnostics.clone();
    let mut ordered = diagnostics.clone();
    sort_diagnostics(&mut ordered);
    let expected_codes = ["E0001", "E1003"];

    let plain = ordered
        .iter()
        .map(|diagnostic| render_diagnostic("source", diagnostic))
        .collect::<Vec<_>>();
    let colored = ordered
        .iter()
        .map(|diagnostic| render_colored_diagnostic("source", diagnostic, true))
        .collect::<Vec<_>>();
    assert!(plain[0].contains(expected_codes[0]) && plain[1].contains(expected_codes[1]));
    assert!(colored[0].contains(expected_codes[0]) && colored[1].contains(expected_codes[1]));

    let encoded = render_json_diagnostics(&diagnostics).expect("JSON renderer should succeed");
    let json: serde_json::Value = serde_json::from_str(&encoded).expect("valid JSON output");
    assert_eq!(json[0]["code"], expected_codes[0]);
    assert_eq!(json[1]["code"], expected_codes[1]);
    assert_eq!(diagnostics, original);

    let plain_result = run_diagnostic_check();
    let colored_result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .env("TERM", "xterm-256color")
        .args(["check", "tests/fixtures/diagnostics/invalid/unexpected_character.act"])
        .output()
        .expect("colored diagnostic check should start");
    assert_eq!(plain_result.status.code(), Some(1));
    assert_eq!(colored_result.status.code(), Some(1));
}

#[test]
fn strict_codes_use_the_reserved_category_ranges() {
    assert_eq!(strict_code_category("E1800"), Some(StrictDiagnosticCategory::Configuration));
    assert_eq!(strict_code_category("E1810"), Some(StrictDiagnosticCategory::Frontend));
    assert_eq!(strict_code_category("E1829"), Some(StrictDiagnosticCategory::Semantic));
    assert_eq!(strict_code_category("E1830"), Some(StrictDiagnosticCategory::Architecture));
    assert_eq!(strict_code_category("E1840"), Some(StrictDiagnosticCategory::Documentation));
    assert_eq!(strict_code_category("E1859"), Some(StrictDiagnosticCategory::Limits));
    assert_eq!(strict_code_category("E1899"), Some(StrictDiagnosticCategory::Execution));
    assert_eq!(strict_code_category("E1079"), None);
    assert_eq!(strict_code_category("E18"), None);
}

#[test]
fn diagnostic_catalog_rejects_duplicate_codes() {
    let diagnostics = vec![
        DiagnosticDefinition::new("E1800", DiagnosticSeverity::Error),
        DiagnosticDefinition::new("E1800", DiagnosticSeverity::Error),
    ];

    assert_eq!(
        validate_diagnostic_catalog(&diagnostics),
        Err(DiagnosticCatalogError::DuplicateCode { code: "E1800".to_owned() })
    );
}

#[test]
fn diagnostic_catalog_rejects_contradictory_severity() {
    let diagnostics = vec![
        DiagnosticDefinition::new("E1800", DiagnosticSeverity::Error),
        DiagnosticDefinition::new("E1800", DiagnosticSeverity::Warning),
    ];

    assert_eq!(
        validate_diagnostic_catalog(&diagnostics),
        Err(DiagnosticCatalogError::ContradictorySeverity {
            code: "E1800".to_owned(),
            first: DiagnosticSeverity::Error,
            second: DiagnosticSeverity::Warning,
        })
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
fn matches_lexical_diagnostic_snapshot() {
    let source = include_str!("fixtures/diagnostics/invalid/unexpected_character.act");
    let (_, errors) = scan(source);
    assert_eq!(errors.len(), 1);
    let actual = render_lex_error(source, &errors[0]);
    let expected = include_str!("fixtures/diagnostics/snapshots/unexpected_character.diag.snap");

    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn matches_parser_diagnostic_snapshot() {
    let source = include_str!("fixtures/parser/invalid/missing_semicolon.act");
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let error = parse(tokens).expect_err("fixture must fail parsing");
    let actual = render_parse_error(source, &error);
    let expected = include_str!("fixtures/diagnostics/snapshots/missing_semicolon.diag.snap");

    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn repeated_clean_checks_produce_identical_diagnostics() {
    let first = run_diagnostic_check();
    let second = run_diagnostic_check();

    assert_eq!(first.status, second.status);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

fn run_diagnostic_check() -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", "tests/fixtures/diagnostics/invalid/unexpected_character.act"])
        .output()
        .expect("diagnostic check should start")
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

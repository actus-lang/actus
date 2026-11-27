//! Bug #130: every `?` the analyzer accepts returns early from the verb. A
//! `?` in a value arm, on the right of `&&` or `||`, in a condition, a `case`
//! subject or a compound assignment is accepted and becomes a statement of its
//! own; a `?` that cannot return early from where it stands, in an argument
//! of a verb or an intrinsic, in the index of an assigned element, or in a
//! conditional nested in the arm of a conditional value, is rejected with
//! `E1016`, never accepted and lowered as the whole `Result`.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

const PRELUDE: &str = "enum Fail { Bad, } \
verb count(erg ok: Bool) -> Result[Int, Fail] { if ok { return Result[Int, Fail].Ok(10); } \
return Result[Int, Fail].Err(Fail.Bad); } \
verb flag(erg ok: Bool) -> Result[Bool, Fail] { if ok { return Result[Bool, Fail].Ok(true); } \
return Result[Bool, Fail].Err(Fail.Bad); } ";

/// Analyzes `body` as the body of `run(ok)`, which returns a `Result`.
fn analyze_run(body: &str) -> Result<(), (String, String, String)> {
    let source = format!(
        "{PRELUDE}verb run(erg ok: Bool) -> Result[Int, Fail] {{ {body} }} verb main() {{ }}"
    );
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    let Err(error) = analyze(&program) else { return Ok(()) };
    let rendered = actus::diagnostics::semantic_diagnostic(&error);
    let span = rendered.span();
    Err((
        rendered.code().to_owned(),
        rendered.message().to_owned(),
        source[span.start..span.end].to_owned(),
    ))
}

#[test]
fn a_try_on_one_path_or_in_a_condition_is_accepted() {
    let ok = "return Result[Int, Fail].Ok(1);";
    for (label, body) in [
        (
            "if arm",
            format!(
                "erg c: Bool = true; erg x: Int = if c {{ count(ok: ok)? }} else {{ 3 }}; {ok}"
            ),
        ),
        (
            "case arm",
            format!("erg k: Int = 1; erg x: Int = case k {{ 1 => count(ok: ok)?, _ => 3, }}; {ok}"),
        ),
        ("and", format!("erg l: Bool = true; erg b: Bool = l && flag(ok: ok)?; {ok}")),
        ("or", format!("erg l: Bool = false; erg b: Bool = l || flag(ok: ok)?; {ok}")),
        ("if condition", format!("if flag(ok: ok)? {{ print(1); }} {ok}")),
        (
            "else if condition",
            format!("erg a: Bool = false; if a {{ }} else if flag(ok: ok)? {{ }} {ok}"),
        ),
        ("case subject", format!("erg x: Int = case count(ok: ok)? {{ 10 => 1, _ => 2, }}; {ok}")),
        ("compound assignment", format!("erg x: Int = 0; x += count(ok: ok)?; {ok}")),
    ] {
        if let Err((code, message, _)) = analyze_run(&body) {
            panic!("{label}: expected acceptance, found {code}: {message}");
        }
    }
}

#[test]
fn a_try_that_cannot_return_early_from_its_place_is_rejected_with_e1016() {
    for (label, body, place) in [
        (
            "intrinsic argument",
            "print(count(ok: ok)?); return Result[Int, Fail].Ok(1);",
            "an argument of `print`",
        ),
        (
            "element index",
            "erg xs: Array[Int, 20] = Array[Int, 20](); xs[count(ok: ok)?] = 1; \
return Result[Int, Fail].Ok(1);",
            "the index of an assigned element",
        ),
        (
            "conditional in a value arm",
            "erg n: Int = 2; erg l: Bool = false; erg b: Bool = n > 1 && (l || flag(ok: ok)?); \
return Result[Int, Fail].Ok(1);",
            "a conditional or a loop nested in the arm of a conditional value",
        ),
    ] {
        let (code, message, spanned) =
            analyze_run(body).expect_err(&format!("{label} must be rejected"));
        assert_eq!(code, "E1016", "{label}: {message}");
        assert!(message.contains(&format!("`?` in {place}")), "{label}: {message}");
        assert!(message.contains("erg value = ...?;"), "{label}: {message}");
        assert!(spanned.ends_with('?'), "{label}: span `{spanned}`");
    }
}

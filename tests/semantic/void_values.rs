//! A call to a verb that returns nothing has no value (E1141): it is only
//! valid as a statement of its own.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

const PRELUDE: &str = "verb greet() { print(string: \"hi\"); } verb explicit() -> Void { } verb sink(erg v: Int) { } verb take(erg v: Int) -> Int { return v; } struct Pair { x: Int, } role Logger { verb note(abs self: Pair); } perform Logger for Pair { verb note(abs self: Pair) { } } ";

fn analyze_with_prelude(body: &str) -> (String, Result<(), actus::semantic::SemanticError>) {
    let source = format!("{PRELUDE}{body}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    let result = analyze(&program).map(|_| ());
    (source, result)
}

/// Asserts that `body` is rejected with E1141 at exactly `call` and naming `verb`.
fn rejects(body: &str, verb: &str, call: &str) {
    let (source, result) = analyze_with_prelude(body);
    let error = result.expect_err("a void call used as a value must be rejected");
    let diagnostic = actus::diagnostics::semantic_diagnostic(&error);
    let span = diagnostic.span();
    assert_eq!(diagnostic.code(), "E1141", "{body}: {}", diagnostic.message());
    assert_eq!(&source[span.start..span.end], call, "{body}");
    assert!(
        diagnostic
            .message()
            .starts_with(&format!("`{verb}` returns nothing, so its call has no value")),
        "{}",
        diagnostic.message()
    );
}

fn accepts(body: &str) {
    let (_, result) = analyze_with_prelude(body);
    result.expect("a void call as a statement must be accepted");
}

#[test]
fn rejects_binding_the_result() {
    rejects("verb main() { erg x = greet(); }", "greet", "greet()");
    rejects("verb main() { erg x: Int = greet(); }", "greet", "greet()");
}

#[test]
fn rejects_assigning_the_result() {
    rejects("verb main() { erg x = 1; x = greet(); }", "greet", "greet()");
    rejects("verb main() { erg x = 1; x += greet(); }", "greet", "greet()");
    rejects("verb main() { erg p = Pair { x: 1 }; p.x = greet(); }", "greet", "greet()");
    rejects(
        "verb main() { erg a: Array[Int, 2] = Array[Int, 2](); a[0] = greet(); }",
        "greet",
        "greet()",
    );
}

#[test]
fn rejects_passing_the_result_as_an_argument() {
    rejects("verb main() { erg y = take(v: greet()); }", "greet", "greet()");
    rejects("verb main() { print(int: greet()); }", "greet", "greet()");
}

#[test]
fn rejects_the_result_as_an_operand() {
    rejects("verb main() { erg y = greet() + 1; }", "greet", "greet()");
    rejects("verb main() { erg y = 1 + greet(); }", "greet", "greet()");
    rejects("verb main() { erg y = -greet(); }", "greet", "greet()");
    rejects("verb main() { erg y = greet() as u8; }", "greet", "greet()");
    rejects("verb main() { greet() + 1; }", "greet", "greet()");
}

#[test]
fn rejects_the_result_in_a_comparison_or_condition() {
    rejects("verb main() { if greet() == 1 { print(string: \"a\"); } }", "greet", "greet()");
    rejects("verb main() { if greet() { print(string: \"a\"); } }", "greet", "greet()");
}

#[test]
fn rejects_returning_the_result() {
    rejects("verb h() -> Int { return greet(); }", "greet", "greet()");
    rejects("verb h() { return greet(); }", "greet", "greet()");
}

#[test]
fn rejects_the_result_as_a_conditional_value() {
    rejects("verb main() { erg x = if true { greet() } else { greet() }; }", "greet", "greet()");
    rejects("verb main() { erg x: Int = if true { 1 } else { greet() }; }", "greet", "greet()");
    rejects("verb main() { erg x: Int = if true { greet() } else { 1 }; }", "greet", "greet()");
    rejects("verb main() { erg x = case 1 { 1 => greet(), _ => greet(), }; }", "greet", "greet()");
}

#[test]
fn rejects_the_result_as_a_struct_field_initializer() {
    rejects("verb main() { erg p = Pair { x: greet() }; }", "greet", "greet()");
}

#[test]
fn rejects_an_explicit_void_verb_the_same_way() {
    rejects("verb main() { erg x = explicit(); }", "explicit", "explicit()");
}

#[test]
fn rejects_the_result_of_a_method_that_returns_nothing() {
    rejects("verb main(abs pair: Pair) { erg x = pair.note(); }", "note", "pair.note()");
    rejects(
        "verb run(abs logger: dynamic Logger) { erg x = logger.note(); }",
        "note",
        "logger.note()",
    );
}

#[test]
fn rejects_the_result_of_an_external_and_a_generic_verb_that_return_nothing() {
    rejects("unsafe extern \"C\" verb poke(); verb main() { erg x = poke(); }", "poke", "poke()");
    rejects(
        "verb ignore[T](erg v: T) { } verb main() { erg x = ignore(v: 1); }",
        "ignore",
        "ignore(v: 1)",
    );
}

#[test]
fn accepts_a_void_call_as_a_statement() {
    accepts("verb main() { greet(); }");
    accepts("verb main() { explicit(); }");
    accepts("verb main() { (greet()); }");
    accepts("verb main(abs pair: Pair) { pair.note(); }");
    accepts("verb main() { if true { greet(); } else if false { greet(); } else { greet(); } }");
    accepts("verb main() { loop { greet(); break; } }");
    accepts("verb main() { case 1 { 1 => greet(), _ => greet(), }; }");
    accepts("verb main() { case 1 { 1 => { greet(); }, _ => { greet(); }, }; }");
    accepts("verb main() { return; }");
}

#[test]
fn still_accepts_value_calls_in_every_position() {
    accepts(
        "verb one() -> Int { return 1; } verb main() { erg x = one(); erg y = one() + 1; one(); }",
    );
}

#[test]
fn accepts_a_void_call_as_the_unit_payload_of_a_result() {
    accepts("verb main() -> Result[Void, String] { return Result[Void, String].Ok(explicit()); }");
}

#[test]
fn rejects_a_void_call_as_a_non_unit_result_payload() {
    rejects(
        "verb main() -> Result[Int, String] { return Result[Int, String].Ok(greet()); }",
        "greet",
        "greet()",
    );
}

#[test]
fn rejects_a_void_call_nested_inside_a_statement_call() {
    rejects("verb main() { sink(v: greet()); }", "greet", "greet()");
    rejects("verb main() { print(int: greet()); }", "greet", "greet()");
    rejects("verb main() { (sink(v: greet())); }", "greet", "greet()");
    rejects("verb main(abs pair: Pair) { sink(v: pair.note()); }", "note", "pair.note()");
    rejects("verb main(abs pair: Pair) { pair.note(); sink(v: greet() + 1); }", "greet", "greet()");
}

#[test]
fn rejects_a_void_call_nested_inside_a_statement_conditional() {
    rejects("verb main() { case 1 { 1 => sink(v: greet()), _ => greet(), }; }", "greet", "greet()");
    rejects(
        "verb main() { if true { greet(); } else if greet() == 1 { greet(); } }",
        "greet",
        "greet()",
    );
    rejects(
        "verb main() { case 1 { 1 => { erg x = greet(); }, _ => greet(), }; }",
        "greet",
        "greet()",
    );
}

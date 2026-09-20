use actus::codegen::emit_program_object;
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn lowers_an_integer_returning_verb_to_a_native_object() {
    let (tokens, errors) = scan("verb main() -> Int { return 42; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("program should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn codegen_rejects_semantically_invalid_programs() {
    let (tokens, errors) =
        scan("verb main() -> Int { erg value = make(); drop(value); inspect(value); }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error =
        emit_program_object(&program, "main").expect_err("semantic errors must block codegen");
    assert!(error.to_string().contains("semantic analysis failed"));
}

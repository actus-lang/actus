use actus::codegen::emit_program_object;
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn lowers_an_integer_returning_verb_to_a_native_object() {
    let (tokens, errors) = scan("verb main() -> Int { erg answer = 40; return answer + 2; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("program should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn emits_identical_objects_for_identical_programs() {
    let source = "verb main() -> Int { erg answer = 40; return answer + 2; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");

    let first = emit_program_object(&program, "main").expect("first emission should pass");
    let second = emit_program_object(&program, "main").expect("second emission should pass");
    assert_eq!(first, second);
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

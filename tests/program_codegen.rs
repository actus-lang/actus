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
fn lowers_integer_parameters_to_a_native_object() {
    let source = "verb add(erg left: Int, erg right: Int) -> Int { return left + right; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "add").expect("integer parameters should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn lowers_integer_function_calls_to_a_native_object() {
    let source = "verb main() -> Int { erg left = 40; erg right = 2; return add(right: right, left: left); } verb add(erg left: Int, erg right: Int) -> Int { return left + right; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("integer call should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn lowers_unary_and_grouped_integer_expressions() {
    let source = "verb main() -> Int { return -(2 + 3) * 4; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object =
        emit_program_object(&program, "main").expect("grouped unary expression should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn lowers_loop_continue_to_a_native_object() {
    let source = "verb main() -> Int { loop { continue; } return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("loop continue should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn lowers_multiple_loop_carried_bindings_to_a_native_object() {
    let source = "verb main() -> Int { erg left = 0; erg right = 0; loop { left = 1; right = 2; break; } return left + right; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("loop-carried values should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn lowers_scalar_borrow_roles_to_a_native_object() {
    let source = "verb main() -> Int { erg value = 42; abs view = ref value; return inspect(view); } verb inspect(abs view: Int) -> Int { return view + 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("scalar borrow should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn lowers_scalar_move_roles_to_a_native_object() {
    let source = "verb main() -> Int { erg value = 41; return consume(value: value); } verb consume(dat value: Int) -> Int { return value + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("scalar move should emit");
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

#[test]
fn codegen_rejects_unsupported_native_return_types() {
    let (tokens, errors) = scan("verb main() -> Buffer { return 42; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = emit_program_object(&program, "main").expect_err("Buffer is not an i32 ABI return");
    assert!(error.to_string().contains("supports `Int` returns"));
}

#[test]
fn codegen_rejects_unsupported_native_parameter_types() {
    let (tokens, errors) = scan("verb main(erg buffer: Buffer) -> Int { return 42; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error =
        emit_program_object(&program, "main").expect_err("Buffer is not an i32 ABI parameter");
    assert!(error.to_string().contains("supports `Int` parameters"));
}

#[test]
fn codegen_rejects_integer_values_outside_native_width() {
    let (tokens, errors) = scan("verb main() -> Int { return 2147483648; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = emit_program_object(&program, "main").expect_err("literal exceeds i32");
    assert!(error.to_string().contains("invalid integer literal"));
}

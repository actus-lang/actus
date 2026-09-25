use actus::codegen::emit_program_object;
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn lowers_an_integer_returning_verb_to_a_native_object() {
    let (tokens, errors) = scan("verb main() -> Int { erg answer = 40; return answer + 2; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("program should emit");
    assert_native_object(&object);
}

#[test]
fn lowers_integer_parameters_to_a_native_object() {
    let source = "verb add(erg left: Int, erg right: Int) -> Int { return left + right; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "add").expect("integer parameters should emit");
    assert_native_object(&object);
}

#[test]
fn lowers_struct_method_calls_to_a_native_object() {
    let source = "struct Point { x: Int, } verb read(abs self: Point) -> Int { return self.x; } verb main() -> Int { erg point = Point { x: 7, }; return point.read(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("struct method should emit");
    assert_native_object(&object);
}

#[test]
fn emits_deterministic_struct_method_objects() {
    let source = "struct Point { x: Int, y: Int, } verb sum(abs self: Point) -> Int { return self.x + self.y; } verb main() -> Int { erg point = Point { x: 7, y: 5, }; return point.sum(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let first = emit_program_object(&program, "main").expect("first emission should pass");
    let second = emit_program_object(&program, "main").expect("second emission should pass");
    assert_eq!(first, second);
}

#[test]
fn emits_deterministic_nested_generic_instances() {
    let source = "struct Box[T] { item: T, } struct Pair[T] { item: T, } verb main() -> Int { erg pair = Pair[Box[Int]] { item: Box[Int] { item: 42, }, }; return pair.item.item; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("nested generic source should parse");
    let first = emit_program_object(&program, "main").expect("first generic emission should pass");
    let second =
        emit_program_object(&program, "main").expect("second generic emission should pass");
    assert_eq!(first, second);
}

#[test]
fn lowers_integer_function_calls_to_a_native_object() {
    let source = "verb main() -> Int { erg left = 40; erg right = 2; return add(right: right, left: left); } verb add(erg left: Int, erg right: Int) -> Int { return left + right; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("integer call should emit");
    assert_native_object(&object);
}

#[test]
fn uses_the_configured_entry_verb_when_it_is_not_first() {
    let source = "verb helper() -> Int { return 1; } verb main() -> Int { return helper(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("configured entry should emit");
    assert_native_object(&object);
}

#[test]
fn lowers_unary_and_grouped_integer_expressions() {
    let source = "verb main() -> Int { return -(2 + 3) * 4; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object =
        emit_program_object(&program, "main").expect("grouped unary expression should emit");
    assert_native_object(&object);
}

#[test]
fn lowers_loop_continue_to_a_native_object() {
    let source = "verb main() -> Int { loop { continue; } return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("loop continue should emit");
    assert_native_object(&object);
}

#[test]
fn lowers_multiple_loop_carried_bindings_to_a_native_object() {
    let source = "verb main() -> Int { erg left = 0; erg right = 0; loop { left = 1; right = 2; break; } return left + right; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("loop-carried values should emit");
    assert_native_object(&object);
}

#[test]
fn lowers_scalar_borrow_roles_to_a_native_object() {
    let source = "verb main() -> Int { erg value = 42; abs view = ref value; return inspect(view); } verb inspect(abs view: Int) -> Int { return view + 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("scalar borrow should emit");
    assert_native_object(&object);
}

#[test]
fn lowers_scalar_move_roles_to_a_native_object() {
    let source = "verb main() -> Int { erg value = 41; return consume(value: value); } verb consume(dat value: Int) -> Int { return value + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("scalar move should emit");
    assert_native_object(&object);
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
fn codegen_accepts_buffer_allocation_and_explicit_drop() {
    let source = "verb main() -> Int { erg buffer: Buffer = Buffer[4]; drop(buffer); return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let object = emit_program_object(&program, "main").expect("buffer operations should emit");
    assert_native_object(&object);
}

#[test]
fn codegen_accepts_buffer_append_calls() {
    let source = "verb main() -> Int { erg buffer: Buffer = Buffer[4]; append(buffer, 42); drop(buffer); return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    emit_program_object(&program, "main").expect("buffer append should emit");
}

#[test]
fn semantic_analysis_rejects_unknown_return_types() {
    let (tokens, errors) = scan("verb main() -> Vector { return 42; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = emit_program_object(&program, "main").expect_err("unknown type must be rejected");
    assert!(error.to_string().contains("UnknownType"));
}

#[test]
fn semantic_analysis_rejects_unknown_parameter_types() {
    let (tokens, errors) = scan("verb main(erg buffer: Vector) -> Int { return 42; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = emit_program_object(&program, "main").expect_err("unknown type must be rejected");
    assert!(error.to_string().contains("UnknownType"));
}

#[test]
fn codegen_rejects_integer_values_outside_native_width() {
    let (tokens, errors) = scan("verb main() -> Int { return 2147483648; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = emit_program_object(&program, "main").expect_err("literal exceeds i32");
    assert!(error.to_string().contains("invalid integer literal"));
}

fn assert_native_object(bytes: &[u8]) {
    object::File::parse(bytes).expect("output should be a native object");
}

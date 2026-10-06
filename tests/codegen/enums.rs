use actus::codegen::emit_program_object;
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn lowers_enum_construction_to_a_native_object() {
    let source =
        "enum Message { Move(Int, Int), } verb main() -> Message { return Message.Move(40, 2); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("enum program should parse");
    let object = emit_program_object(&program, "main").expect("enum construction should emit");
    object::File::parse(object.as_slice()).expect("enum construction should emit a native object");
}

#[test]
fn emits_deterministic_enum_layout_objects() {
    let source = "enum Value { Empty, Count(Int), Text(String), } verb main() -> Value { return Value.Count(42); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("enum program should parse");
    let first = emit_program_object(&program, "main").expect("first enum emission should pass");
    let second = emit_program_object(&program, "main").expect("second enum emission should pass");
    assert_eq!(first, second);
}

#[test]
fn lowers_enum_case_discriminant_switch_to_native_code() {
    let source = "enum Color { Red, Green, } verb main(erg color: Color) -> Int { return case color { Color.Red => 1, Color.Green => 2, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("enum case program should parse");
    let object = emit_program_object(&program, "main").expect("enum case should emit");
    object::File::parse(object.as_slice()).expect("enum case should emit a native object");
}

#[test]
fn lowers_hex_integer_case_patterns_without_defaulting_to_zero() {
    let source = "verb main(erg value: Int) -> Int { return case value { 0x10 => 1, _ => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("hex pattern program should parse");
    let object = emit_program_object(&program, "main").expect("hex pattern should emit");
    object::File::parse(object.as_slice()).expect("hex pattern should emit a native object");
}

#[test]
fn lowers_pattern_guard_failure_to_the_next_case_branch() {
    let source = "enum Color { Red, Green, } verb main(erg ready: Bool, erg color: Color) -> Int { return case color { Color.Red if ready => 1, _ => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("guarded case program should parse");
    let object = emit_program_object(&program, "main").expect("guarded case should emit");
    object::File::parse(object.as_slice()).expect("guarded case should emit a native object");
}

#[test]
fn lowers_case_payload_bindings_to_native_loads() {
    let source = "enum Message { Move(Int, Int), } verb main(erg message: Message) -> Int { return case dat message { Message.Move(x, y) => x + y, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("payload case program should parse");
    let object = emit_program_object(&program, "main").expect("payload case should emit");
    object::File::parse(object.as_slice()).expect("payload case should emit a native object");
}

#[test]
fn lowers_case_block_body_with_return_cleanup_path() {
    let source = "enum Message { Move(Int, Int), } verb main(erg message: Message) -> Int { return case dat message { Message.Move(x, y) => { return x + y; }, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("case block program should parse");
    let object = emit_program_object(&program, "main").expect("case block should emit");
    object::File::parse(object.as_slice()).expect("case block should emit a native object");
}

#[test]
fn lowers_mixed_case_body_forms_with_explicit_return_path() {
    let source =
        "verb main(erg value: Int) -> Int { return case value { 0 => 1, _ => { return 2; }, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("mixed case body program should parse");
    let object = emit_program_object(&program, "main")
        .expect("mixed case body forms with a returning block should emit");
    object::File::parse(object.as_slice()).expect("mixed case body should emit a native object");
}

#[test]
fn lowers_case_payload_cleanup_without_double_drop() {
    let source = "enum Message { Move { payload: Buffer, keep: Buffer, }, } verb main(erg message: Message) -> Int { return case dat message { Message.Move(payload: moved, keep: _) => { drop(moved); return 1; }, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("cleanup case program should parse");
    let object = emit_program_object(&program, "main").expect("cleanup case should emit");
    object::File::parse(object.as_slice()).expect("cleanup case should emit a native object");
}

#[test]
fn lowers_generic_option_case_to_native_code() {
    let source = "verb main() -> Int { erg option = Option[Int].Some(42); return case dat option { Option.Some(value) => value, Option.None => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generic Option program should parse");
    let object = emit_program_object(&program, "main").expect("generic Option should emit");
    object::File::parse(object.as_slice()).expect("generic Option should emit a native object");
}

#[test]
fn lowers_generic_result_case_to_native_code() {
    let source = "verb main() -> Int { erg result = Result[Int, String].Ok(42); return case dat result { Result.Ok(value) => value, Result.Err(_) => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generic Result program should parse");
    let object = emit_program_object(&program, "main").expect("generic Result should emit");
    object::File::parse(object.as_slice()).expect("generic Result should emit a native object");
}

#[test]
fn emits_deterministic_generic_result_objects() {
    let source = "verb main() -> Int { erg result = Result[Int, String].Ok(42); return case dat result { Result.Ok(value) => value, Result.Err(_) => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generic Result program should parse");
    let first =
        emit_program_object(&program, "main").expect("first generic Result emission should pass");
    let second =
        emit_program_object(&program, "main").expect("second generic Result emission should pass");
    assert_eq!(first, second);
    object::File::parse(first.as_slice()).expect("generic Result should emit a native object");
}

#[test]
fn lowers_owned_generic_result_cases_with_borrowed_and_exclusive_arguments() {
    let source = "enum Failure { Failed, } verb produce(erg ready: Bool) -> Result[Buffer, Failure] { if ready { return Result[Buffer, Failure].Ok(Buffer[4]); } return Result[Buffer, Failure].Err(Failure.Failed); } verb dispose(dat result: Result[Buffer, Failure]) -> Int { return case dat result { Result.Ok(payload) => { drop(payload); return 1; }, Result.Err(_) => { return 0; }, }; } verb consume(abs input: Buffer, ins state: Buffer, erg ready: Bool) -> Int { erg first = produce(ready: erg ready); erg second = produce(ready: erg ready); erg first_code = dispose(result: dat first); erg second_code = dispose(result: dat second); return buffer_length(buffer: abs input) + buffer_length(buffer: abs state) as Int + first_code + second_code; } verb main() -> Int { erg input = Buffer[4]; erg state = Buffer[4]; erg ready = true; return consume(input: abs input, state: ins state, ready: erg ready); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("owned generic Result program should parse");
    let object =
        emit_program_object(&program, "main").expect("owned generic Result cases should emit");
    object::File::parse(object.as_slice()).expect("owned generic Result object should parse");
}

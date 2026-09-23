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

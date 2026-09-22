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
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
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
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn lowers_case_payload_bindings_to_native_loads() {
    let source = "enum Message { Move(Int, Int), } verb main(erg message: Message) -> Int { return case dat message { Message.Move(x, y) => x + y, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("payload case program should parse");
    let object = emit_program_object(&program, "main").expect("payload case should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

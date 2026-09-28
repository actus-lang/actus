use actus::codegen::emit_program_object;
use actus::lexer::scan;
use actus::parser::parse;

fn emit_layout_fixture(source: &str) {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("layout fixture should parse");
    emit_program_object(&program, "main").expect("layout fixture should lower");
}

#[test]
fn lowers_natural_and_nested_struct_layouts() {
    emit_layout_fixture(
        "struct Inner { x: Int, y: Int, } struct Outer { inner: Inner, tag: Int, } verb main() -> Int { return 0; }",
    );
}

#[test]
fn lowers_enum_payload_and_unit_layouts() {
    emit_layout_fixture(
        "enum Value { Empty, Count(Int), Text(String), Pair { left: Int, right: Int, }, } enum Color { Red, Green, } verb main() -> Int { return 0; }",
    );
}

#[test]
fn preserves_pointer_niche_for_recursive_reference_options() {
    emit_layout_fixture(
        "struct Node { value: Int, next: Option[abs Node], } verb main() -> Int { return 0; }",
    );
}

#[test]
fn uses_a_caller_slot_for_borrowed_descriptor_options() {
    emit_layout_fixture(
        "struct View { abs source: Buffer, offset: Int, length: Int, } unsafe extern \"C\" verb find(abs source: View) -> Option[abs View]; verb main() -> Int { return 0; }",
    );
}

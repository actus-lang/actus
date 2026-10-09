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
fn lowers_region_layouts_for_primitive_array_struct_and_pack_elements() {
    emit_layout_fixture(
        "struct Point { x: u32, y: u32, } struct Inner { left: u16, right: u32, } struct Outer { prefix: u8, inner: Inner, suffix: u64, } pack Frame { erg storage: Array[u8, 16]; layout big; fields { erg marker: u8 at 0; abs _reserved: u120 at 8 = 0; } } verb inspect(abs primitive: Region[u32], abs array: Region[Array[u8, 16]], abs nested: Region[Array[Array[u8, 4], 4]], abs point: Region[Point], abs outer: Region[Outer], abs frame: Region[Frame]) { } verb main() -> Int { return 0; }",
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

use actus::codegen::emit_zero_return_object;
use object::{Object, ObjectSection, ObjectSymbol};

#[test]
fn emits_a_native_object_without_c_intermediate_code() {
    let object = emit_zero_return_object("actus_entry").expect("native object should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
    assert!(object.len() > 256);
}

#[test]
fn native_object_contains_a_stable_entry_symbol_and_code_section() {
    let bytes = emit_zero_return_object("actus_entry").expect("native object should emit");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();

    assert!(symbols.contains(&"actus_entry"));
    assert!(file.sections().any(|section| section.kind() == object::SectionKind::Text));
}

use actus::codegen::emit_zero_return_object;

#[test]
fn emits_a_native_object_without_c_intermediate_code() {
    let object = emit_zero_return_object("actus_entry").expect("native object should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
    assert!(object.len() > 256);
}

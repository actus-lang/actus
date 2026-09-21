use actus::codegen::emit_zero_return_object;
use actus::lexer::scan;
use actus::parser::parse;
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

#[cfg(target_arch = "x86_64")]
#[test]
fn x86_64_machine_code_matches_the_return_zero_golden() {
    let (tokens, errors) = scan("verb main() -> Int { return 0; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let bytes = actus::codegen::emit_program_object(&program, "main").expect("object should emit");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let text = file
        .sections()
        .find(|section| section.kind() == object::SectionKind::Text)
        .expect("text section should exist")
        .data()
        .expect("text section should have bytes");

    assert_eq!(text, [0x55, 0x48, 0x89, 0xe5, 0x33, 0xc0, 0x48, 0x89, 0xec, 0x5d, 0xc3]);
}

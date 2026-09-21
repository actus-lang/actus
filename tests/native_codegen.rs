use actus::codegen::{emit_program_object_with_configuration, emit_zero_return_object};
use actus::configuration::NativeBackendConfiguration;
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

#[test]
fn native_backend_accepts_externalized_build_settings() {
    let (tokens, errors) = scan("verb main() -> Int { return 42; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let configuration = NativeBackendConfiguration::new("actus-test-module", false);

    let object = emit_program_object_with_configuration(&program, "main", &configuration)
        .expect("custom native settings should emit");
    assert!(object.starts_with(b"\x7fELF") || object.starts_with(b"\xcf\xfa\xed\xfe"));
}

#[test]
fn native_backend_rejects_registered_but_unlowered_collection_types() {
    let (tokens, errors) = scan("verb main(erg items: Array) -> Int { return 0; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = actus::codegen::emit_program_object(&program, "main")
        .expect_err("unlowered collection types must be rejected");

    assert!(error.to_string().contains("Array"));
}

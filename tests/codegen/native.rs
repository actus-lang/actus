use actus::codegen::{emit_program_object_with_configuration, emit_zero_return_object};
use actus::configuration::NativeBackendConfiguration;
use actus::lexer::scan;
use actus::parser::parse;
use object::{Object, ObjectSection, ObjectSymbol};

#[test]
fn emits_a_native_object_without_c_intermediate_code() {
    let object = emit_zero_return_object("actus_entry").expect("native object should emit");
    object::File::parse(object.as_slice()).expect("object format should parse");
}

#[test]
fn native_object_contains_a_stable_entry_symbol_and_code_section() {
    let bytes = emit_zero_return_object("actus_entry").expect("native object should emit");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();

    assert!(symbols.iter().any(|symbol| symbol_matches(symbol, "actus_entry")));
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
    object::File::parse(object.as_slice()).expect("object format should parse");
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

#[test]
fn declares_external_c_functions_as_imported_symbols() {
    let source = "unsafe extern \"C\" verb rand() -> Int; verb main() -> Int { return rand(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let bytes = actus::codegen::emit_program_object(&program, "main")
        .expect("external calls should emit an object");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();

    assert!(symbols.iter().any(|symbol| symbol_matches(symbol, "rand")));
}

#[test]
fn emits_a_deterministic_role_vtable_data_object() {
    let source = "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return self.value; } } verb main() -> Int { erg file = File { value: 1, }; return file.write(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("role performance source should parse");
    let bytes = actus::codegen::emit_program_object(&program, "main")
        .expect("role vtable data should emit");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();

    assert!(symbols.iter().any(|symbol| {
        symbol.strip_prefix('_').is_some_and(|name| name.starts_with("actus_vtable_Writer_struct_"))
            || symbol.starts_with("actus_vtable_Writer_struct_")
    }));
}

#[test]
fn lowers_dynamic_role_call_through_fat_pointer_abi() {
    let source = "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return self.value + 1; } } verb send(abs writer: dynamic Writer) -> Int { return writer.write(); } verb main() -> Int { erg file = File { value: 41, }; return send(writer: ref file); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("dynamic source should parse");
    let object = actus::codegen::emit_program_object(&program, "main")
        .expect("dynamic call should lower to native code");
    object::File::parse(object.as_slice()).expect("native object should parse");
}

#[test]
fn lowers_ins_parameters_without_a_wrapper_or_extra_allocation() {
    let source = "verb mutate(ins buffer: Buffer) -> Int { return 0; } verb main() -> Int { erg buffer = Buffer[1]; return mutate(buffer: ins buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("ins source should parse");
    let bytes = actus::codegen::emit_program_object(&program, "main")
        .expect("ins parameter should use the ordinary native argument representation");
    let file = object::File::parse(bytes.as_slice()).expect("native object should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(symbols.iter().any(|symbol| symbol_matches(symbol, "mutate")));
}

#[test]
fn rejects_ins_aliasing_before_native_lowering() {
    let source = "verb merge(ins left: Buffer, abs view: Buffer) -> Int { return 0; } verb main() -> Int { erg buffer = Buffer[1]; return merge(left: ins buffer, view: abs buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("aliasing source should parse");
    let error = actus::codegen::emit_program_object(&program, "main")
        .expect_err("semantic aliasing must stop native lowering");
    assert!(error.to_string().contains("ExclusiveLoanAlias"));
}

fn symbol_matches(actual: &str, expected: &str) -> bool {
    actual == expected || actual.strip_prefix('_') == Some(expected)
}

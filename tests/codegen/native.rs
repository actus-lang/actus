#[cfg(not(target_os = "macos"))]
use actus::codegen::emit_program_object_for_target;
use actus::codegen::{emit_program_object_with_configuration, emit_zero_return_object};
use actus::configuration::NativeBackendConfiguration;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;
use actus::target::TargetSpec;
use object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};

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
fn test_zero_float_verification_inspects_generated_integer_ir() {
    let (tokens, errors) =
        scan("verb main() -> Int { erg value: u32 = 1u32; return value + 1u32; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("integer source should parse");
    let configuration = NativeBackendConfiguration::default().with_no_float_ir_verification();

    emit_program_object_with_configuration(&program, "main", &configuration)
        .expect("integer-only generated IR should pass the float audit");
}

#[test]
fn generated_scalar_locals_have_no_allocation_relocations() {
    let source = "verb read(abs value: u32) -> Int { return value as Int; } verb main() -> Int { erg index: u32 = 41u32; return read(value: abs (index + 1u32)); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generated scalar source should parse");
    let bytes = emit_program_object_with_configuration(
        &program,
        "main",
        &NativeBackendConfiguration::default().with_no_float_ir_verification(),
    )
    .expect("generated scalar local should emit natively");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let relocation_names = file
        .sections()
        .flat_map(|section| section.relocations())
        .filter_map(|(_, relocation)| match relocation.target() {
            RelocationTarget::Symbol(index) => file.symbol_by_index(index).ok(),
            _ => None,
        })
        .filter_map(|symbol| symbol.name().ok())
        .collect::<Vec<_>>();
    assert!(!relocation_names.iter().any(|name| name.contains("allocate")));
}

#[test]
fn lowers_constant_abs_scalar_arguments_before_native_inlining() {
    let source = "const VERSION: u16 = 7u16; verb append_u16(ins output: Buffer, abs value: u16) { append(output, value as u8); } verb main() -> Int { erg output: Buffer = Buffer[0]; append_u16(output: ins output, value: abs VERSION); return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("constant role source should parse");
    emit_program_object_with_configuration(
        &program,
        "main",
        &NativeBackendConfiguration::default().with_no_float_ir_verification(),
    )
    .expect("constant-qualified abs scalar argument should emit natively");
}

#[test]
fn generated_and_explicit_scalar_locals_emit_identical_text() {
    let sources = [
        "verb read(abs value: u32) -> Int { return value as Int; } verb main() -> Int { erg index: u32 = 41u32; return read(value: abs (index + 1u32)); }",
        "verb read(abs value: u32) -> Int { return value as Int; } verb main() -> Int { erg index: u32 = 41u32; erg next: u32 = index + 1u32; return read(value: abs next); }",
    ];
    let text_sections = sources.map(|source| {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let program = parse(tokens).expect("scalar parity source should parse");
        let bytes = emit_program_object_with_configuration(
            &program,
            "main",
            &NativeBackendConfiguration::default().with_no_float_ir_verification(),
        )
        .expect("scalar parity source should emit");
        let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
        file.sections()
            .find(|section| section.kind() == object::SectionKind::Text)
            .expect("text section should exist")
            .data()
            .expect("text section should have data")
            .to_vec()
    });
    assert_eq!(text_sections[0], text_sections[1]);
}

#[test]
fn zero_float_verification_rejects_generated_float_ir() {
    let (tokens, errors) = scan("verb main() -> Int { erg value: f32 = 1.0f32; return 0; }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("float source should parse");
    let configuration = NativeBackendConfiguration::default().with_no_float_ir_verification();

    let error = emit_program_object_with_configuration(&program, "main", &configuration)
        .expect_err("float IR must be rejected by the zero-float contract");
    assert!(error.to_string().contains("native function `main`"));
    assert!(error.to_string().contains("floating-point IR instructions"));
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
    let source = "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return self.value; } } verb main() -> Int { erg source = File { value: 1, }; abs file = ref source; return file.write(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("role performance source should parse");
    let bytes = actus::codegen::emit_program_object(&program, "main")
        .expect("role vtable data should emit");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();

    assert!(
        symbols
            .iter()
            .any(|symbol| symbol_matches(symbol, "actus_root__vtable_Writer__struct_5f0"))
    );
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
    assert!(symbols.iter().any(|symbol| symbol_matches(symbol, "actus_root__verb_mutate")));
}

#[test]
fn lowers_region_owner_cleanup_to_the_runtime_release_bridge() {
    let source = "verb main(erg region: Region[u32]) -> Int { return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("region source should parse");
    let bytes = actus::codegen::emit_program_object_with_configuration(
        &program,
        "main",
        &NativeBackendConfiguration::default().with_no_float_ir_verification(),
    )
    .expect("region owner cleanup should lower natively");
    let file = object::File::parse(bytes.as_slice()).expect("native object should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(symbols.iter().any(|symbol| symbol_matches(symbol, "actus_region_drop")));
}

#[test]
fn lowers_size_of_to_a_compile_time_integer_constant() {
    let source = "verb main() -> u64 { return size_of[u32](); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("size_of source should parse");
    let bytes = actus::codegen::emit_program_object_with_configuration(
        &program,
        "main",
        &NativeBackendConfiguration::default().with_no_float_ir_verification(),
    )
    .expect("size_of should lower without a runtime dependency");
    let file = object::File::parse(bytes.as_slice()).expect("native object should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(!symbols.iter().any(|symbol| symbol.contains("size_of")));
}

#[test]
fn keeps_region_symbols_bounded_by_logical_capacity() {
    let source = "verb inspect(abs region: Region[Array[u8, 1048576]]) -> Int { return 0; } verb main() -> Int { return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("large logical region source should parse");
    let bytes = actus::codegen::emit_program_object_with_configuration(
        &program,
        "main",
        &NativeBackendConfiguration::default().with_no_float_ir_verification(),
    )
    .expect("large logical capacity must not specialize native symbols");
    let file = object::File::parse(bytes.as_slice()).expect("native object should parse");
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(!symbols.iter().any(|symbol| symbol.contains("1048576")));
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

#[test]
fn pack_source_is_valid_for_freestanding_target_contract() {
    let source = "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } verb main() -> Int { erg register = Register { storage: 0, }; register.enabled = 1; return register.enabled; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("pack source should parse");
    let target =
        TargetSpec::parse("x86_64-unknown-uefi").expect("freestanding target should parse");
    assert_eq!(target.entry_contract(), actus::target::EntryContract::Freestanding);
    analyze(&program).expect("pack should be semantically valid for a freestanding target");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn arena_lowering_is_freestanding_and_has_no_host_runtime_imports() {
    let source = "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[128] = Arena[128](); erg node = arena.place(value: Node { value: 7, }); return node.value; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("arena source should parse");
    let target =
        TargetSpec::parse("x86_64-unknown-uefi").expect("freestanding target should parse");
    let bytes = emit_program_object_for_target(
        &program,
        "main",
        &NativeBackendConfiguration::default(),
        &target,
    )
    .expect("arena lowering should not require host runtime services");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let undefined = file
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name().ok())
        .collect::<Vec<_>>();
    assert!(undefined.is_empty(), "freestanding arena object imports host symbols: {undefined:?}");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn array_and_pack_lowering_is_freestanding_and_has_no_host_runtime_imports() {
    let source = "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } verb main() -> Int { erg registers: Array[Register, 2] = Array[Register, 2](); registers[1] = Register { storage: 0, }; registers[1].enabled = 1; return registers[1].enabled; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("array and pack source should parse");
    let target =
        TargetSpec::parse("x86_64-unknown-uefi").expect("freestanding target should parse");
    let bytes = emit_program_object_for_target(
        &program,
        "main",
        &NativeBackendConfiguration::default(),
        &target,
    )
    .expect("array and pack lowering should not require host runtime services");
    let file = object::File::parse(bytes.as_slice()).expect("object format should parse");
    let undefined = file
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name().ok())
        .collect::<Vec<_>>();
    assert!(undefined.is_empty(), "freestanding array object imports host symbols: {undefined:?}");
}

#[cfg(target_os = "macos")]
#[test]
fn arena_freestanding_contract_is_valid_on_macos() {
    let source = "struct Node { value: Int, } verb main() -> Int { erg arena: Arena[128] = Arena[128](); erg node = arena.place(value: Node { value: 7, }); return node.value; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("arena source should parse");
    let target =
        TargetSpec::parse("x86_64-unknown-uefi").expect("freestanding target should parse");
    assert_eq!(target.entry_contract(), actus::target::EntryContract::Freestanding);
    analyze(&program).expect("arena should be semantically valid for a freestanding target");
}

fn symbol_matches(actual: &str, expected: &str) -> bool {
    actual == expected || actual.strip_prefix('_') == Some(expected)
}

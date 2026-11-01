use super::*;

#[test]
fn shared_lowered_module_is_consumed_by_llvm_and_cranelift() {
    let source = "verb main() -> Int { return 40 + 2; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("shared backend fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("source should lower through the shared contract");

    let llvm_ir = emit_module(&module).expect("LLVM should consume the shared module");
    assert!(llvm_ir.contains("define i32 @main()"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let cranelift_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should consume the shared module");
    let object = object::File::parse(cranelift_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "main"));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_emits_recursive_direct_call() {
    let source = "verb countdown(erg value: Int) -> Int { if value == 0 { return 0; } erg next: Int = value - 1; return countdown(value: erg next); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "recursive fixture should lex: {errors:?}");
    let program = parse(tokens).expect("recursive fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("countdown"))
        .expect("recursive call should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should emit recursive calls");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "countdown"));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_names_contract_string_data_per_function() {
    let source = "verb first() -> String { return \"shared\"; } verb second() -> String { return \"shared\"; } verb main() -> Int { return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "contract string fixture should lex: {errors:?}");
    let program = parse(tokens).expect("contract string fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("contract string fixture should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    emit_contract_module_object(&module, &NativeBackendConfiguration::default(), &target)
        .expect("Cranelift should keep contract string data symbols unique");
}

#[cfg(feature = "cranelift")]
#[test]
fn both_backends_consume_shared_fixed_array_iteration() {
    let source = "verb inspect(abs values: Array[u32, 4]) -> Int { erg total: u32 = 0u32; for erg index: u32 in values { total += values[index]; } return total as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "fixed-array fixture should lex: {errors:?}");
    let program = parse(tokens).expect("fixed-array fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("inspect"))
        .expect("fixed-array iteration should lower through the shared contract");

    let llvm_ir = emit_module(&module).expect("LLVM should consume fixed-array iteration");
    assert!(llvm_ir.contains("array.element.load"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should consume fixed-array iteration");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "inspect"));
}

#[test]
fn llvm_consumes_shared_array_pack_bit_fields() {
    let source = "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } verb main() -> Int { erg registers: Array[Register, 2] = Array[Register, 2](); registers[1] = Register { storage: 0, }; registers[1].enabled = 1; return registers[1].enabled; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "packed field fixture should lex: {errors:?}");
    let program = parse(tokens).expect("packed field fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("packed fields should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should consume packed field instructions");
    assert!(llvm_ir.contains("packed.field.load"));
    assert!(llvm_ir.contains("packed.field.updated"));
}

#[cfg(feature = "cranelift")]
#[test]
fn both_backends_consume_repr_packed_unaligned_fields() {
    let source = "meta repr(C, packed) struct Wire { prefix: u8, value: u32, } verb main() -> Int { erg wire: Wire = Wire { prefix: 1u8, value: 42u32, }; wire.value = 43u32; return wire.value as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "repr(packed) fixture should lex: {errors:?}");
    let program = parse(tokens).expect("repr(packed) fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("repr(packed) fields should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should consume repr(packed) fields");
    assert!(llvm_ir.contains("align 1"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let object_bytes =
        emit_contract_module_object(&module, &NativeBackendConfiguration::default(), &target)
            .expect("Cranelift should consume repr(packed) fields");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "main"));
}

#[cfg(feature = "cranelift")]
#[test]
fn both_backends_consume_repr_transparent_field_access() {
    let source = "meta repr(transparent) struct UserId { value: u64, } verb main() -> Int { erg id: UserId = UserId { value: 41u64, }; id.value = 42u64; return id.value as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "repr(transparent) fixture should lex: {errors:?}");
    let program = parse(tokens).expect("repr(transparent) fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("transparent fields should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should consume transparent fields");
    assert!(llvm_ir.contains("define"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let object_bytes =
        emit_contract_module_object(&module, &NativeBackendConfiguration::default(), &target)
            .expect("Cranelift should consume transparent fields");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "main"));
}

#[cfg(feature = "cranelift")]
#[test]
fn both_backends_consume_repr_explicit_alignment() {
    let source = "meta repr(C, align(8u32)) struct Aligned { value: u8, } verb main() -> Int { erg value: Aligned = Aligned { value: 7u8, }; return value.value as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "repr(align) fixture should lex: {errors:?}");
    let program = parse(tokens).expect("repr(align) fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("repr(align) should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should consume repr(align)");
    assert!(llvm_ir.contains("define"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let object_bytes =
        emit_contract_module_object(&module, &NativeBackendConfiguration::default(), &target)
            .expect("Cranelift should consume repr(align)");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "main"));
}

#[cfg(feature = "cranelift")]
#[test]
fn both_backends_consume_repr_c_payload_enum() {
    let source = "meta repr(C) enum Message { Byte { value: u8, }, Pair { code: u16, value: u32, }, Empty, } verb main() -> Int { erg message = Message.Pair(code: 7u16, value: 11u32); return case dat message { Message.Pair { code: code_value, value: value_value } => code_value as Int + value_value as Int, _ => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "repr(C) payload fixture should lex: {errors:?}");
    let program = parse(tokens).expect("repr(C) payload fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("repr(C) payload enum should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should consume repr(C) payload enum");
    assert!(llvm_ir.contains("define"));

    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let object_bytes =
        emit_contract_module_object(&module, &NativeBackendConfiguration::default(), &target)
            .expect("Cranelift should consume repr(C) payload enum");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "main"));
}

#[test]
fn both_backends_emit_user_defined_drop_performance_calls() {
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { return; } } verb inspect() { erg value: Counter = Counter { value: 7, }; return; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Drop performance fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Drop performance fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("inspect"))
        .expect("Drop performance fixture should lower through the shared contract");

    let llvm_ir = emit_module(&module).expect("LLVM should emit the user-defined Drop call");
    assert!(llvm_ir.contains("define void @actus_root__performance_drop__Drop_Counter"));
    assert!(llvm_ir.contains("call void @actus_root__performance_drop__Drop_Counter"));

    #[cfg(feature = "cranelift")]
    {
        let target = actus::target::TargetSpec::host().expect("host target should parse");
        let configuration = NativeBackendConfiguration::default();
        let object_bytes = emit_contract_module_object(&module, &configuration, &target)
            .expect("Cranelift should emit the user-defined Drop call");
        let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
        assert!(object.symbols().any(|symbol| {
            symbol_name_matches(
                object.format(),
                symbol.name().unwrap_or_default(),
                "actus_root__performance_drop__Drop_Counter",
            )
        }));
    }
}

#[test]
fn both_backends_emit_scalar_aggregate_field_access() {
    let source = "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; return counter.value + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "aggregate field fixture should lex: {errors:?}");
    let program = parse(tokens).expect("aggregate field fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("aggregate field access should lower through the shared contract");

    let llvm_ir = emit_module(&module).expect("LLVM should emit aggregate field access");
    assert!(llvm_ir.contains("aggregate.field.load"));

    #[cfg(feature = "cranelift")]
    {
        let target = actus::target::TargetSpec::host().expect("host target should parse");
        let configuration = NativeBackendConfiguration::default();
        let object_bytes = emit_contract_module_object(&module, &configuration, &target)
            .expect("Cranelift should emit aggregate field access");
        let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
        assert!(has_symbol(&object, "main"));
    }
}

#[test]
fn both_backends_emit_scalar_aggregate_field_assignment() {
    let source = "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value = 42; return counter.value; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "aggregate field assignment should lex: {errors:?}");
    let program = parse(tokens).expect("aggregate field assignment should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("aggregate field assignment should lower through the shared contract");
    let llvm_ir = emit_module(&module).expect("LLVM should emit aggregate field assignment");
    assert!(llvm_ir.contains("aggregate.field.load"));
    #[cfg(feature = "cranelift")]
    {
        let target = actus::target::TargetSpec::host().expect("host target should parse");
        let configuration = NativeBackendConfiguration::default();
        let object_bytes = emit_contract_module_object(&module, &configuration, &target)
            .expect("Cranelift should emit aggregate field assignment");
        let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
        assert!(has_symbol(&object, "main"));
    }
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_consumes_scalar_result_case_from_shared_lowering() {
    let source = "verb inspect(dat result: Result[Int, Int]) -> Int { return case dat result { Result.Ok(value) => value, Result.Err(error) => error, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Result fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Result fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("inspect"))
        .expect("scalar Result case should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should consume scalar Result extraction");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "inspect"));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_emits_scalar_result_constructor_to_return_slot() {
    let source = "verb main() -> Result[Int, Int] { return Result[Int, Int].Ok(7); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Result constructor should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Result constructor should parse");
    let module = lower_scalar_program("actus_root", &program, Some("main"))
        .expect("Result constructor should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should emit scalar Result construction");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "main"));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_emits_region_result_constructor_to_return_slot() {
    let source = "verb wrap(dat region: Region[u32]) -> Result[Region[u32], Int] { return Result[Region[u32], Int].Ok(region); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Region Result fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Region Result fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("wrap"))
        .expect("Region Result constructor should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should emit a Region Result constructor");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "wrap"));
}

#[cfg(feature = "cranelift")]
#[test]
fn cranelift_consumes_region_result_case_from_shared_lowering() {
    let source = "verb inspect(dat result: Result[Region[u32], Int]) -> Int { return case dat result { Result.Ok(value) => 0, Result.Err(error) => error, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Cranelift Region case fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Cranelift Region case fixture should parse");
    let module = lower_scalar_program("actus_root", &program, Some("inspect"))
        .expect("Region Result case should lower through the shared contract");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = NativeBackendConfiguration::default();
    let object_bytes = emit_contract_module_object(&module, &configuration, &target)
        .expect("Cranelift should consume a Region Result case");
    let object = object::File::parse(object_bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "inspect"));
}

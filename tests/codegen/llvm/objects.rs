use super::*;

#[test]
fn emits_a_freestanding_llvm_object_without_host_runtime_imports() {
    let source = "verb main() -> Int { erg value: u32 = 41u32; return value as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "freestanding fixture should lex: {errors:?}");
    let program = parse(tokens).expect("freestanding fixture should parse");
    let module = lower_scalar_program("freestanding_test", &program, Some("main"))
        .expect("freestanding fixture should lower through the neutral contract");
    let target = actus::target::TargetSpec::parse("x86_64-unknown-uefi")
        .expect("freestanding target should parse");
    let bytes =
        emit_object_bytes_with_configuration_and_float_policy(&module, &target, false, true)
            .expect("freestanding LLVM object should emit");
    let object = object::File::parse(bytes.as_slice()).expect("freestanding object should parse");
    assert_eq!(object.architecture(), object::Architecture::X86_64);
    let undefined = object
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name().ok())
        .collect::<Vec<_>>();
    assert!(undefined.is_empty(), "freestanding LLVM object imports host symbols: {undefined:?}");
}

#[test]
fn emits_a_host_object_with_a_stable_public_symbol() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: Vec::new(),
            }],
            cleanup: Vec::new(),
        }],
    };
    let output = std::env::temp_dir().join(format!("actus-llvm-object-{}.o", std::process::id()));
    let second_output =
        std::env::temp_dir().join(format!("actus-llvm-object-{}-second.o", std::process::id()));
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    emit_object(&module, &target, &output).expect("host object should be emitted");
    emit_object(&module, &target, &second_output).expect("second host object should be emitted");
    let bytes = std::fs::read(&output).expect("object should be readable");
    let second_bytes = std::fs::read(&second_output).expect("second object should be readable");
    assert_eq!(bytes, second_bytes, "repeated object emission must be deterministic");
    let object = object::File::parse(bytes.as_slice()).expect("object should parse");
    assert!(has_symbol(&object, "test__verb_main"));
    let configured_output =
        std::env::temp_dir().join(format!("actus-llvm-object-{}-configured.o", std::process::id()));
    emit_object_with_configuration_and_options(
        &module,
        &target,
        &configured_output,
        true,
        ObjectEmissionOptions {
            emit_stack_size_section: true,
            function_sections: true,
            ..ObjectEmissionOptions::default()
        },
    )
    .expect("configured host object should be emitted");
    let configured_bytes =
        std::fs::read(&configured_output).expect("configured object should read");
    let configured =
        object::File::parse(configured_bytes.as_slice()).expect("configured object should parse");
    if target.object_format == target_lexicon::BinaryFormat::Elf {
        assert!(
            configured.section_by_name(".stack_sizes").is_some(),
            "configured ELF object must publish .stack_sizes"
        );
        let text_sections = configured
            .sections()
            .filter(|section| section.kind() == object::SectionKind::Text)
            .filter_map(|section| section.name().ok())
            .collect::<Vec<_>>();
        assert!(
            text_sections.iter().any(|name| name.contains("test__verb_main")),
            "configured ELF object must isolate the function section: {text_sections:?}"
        );
    } else {
        assert_eq!(
            configured_bytes, bytes,
            "non-ELF configured options must preserve default object bytes"
        );
        assert!(
            configured.section_by_name(".stack_sizes").is_none(),
            "non-ELF targets must not enable the ELF frame-report option"
        );
    }
    let disabled_output =
        std::env::temp_dir().join(format!("actus-llvm-object-{}-disabled.o", std::process::id()));
    emit_object_with_configuration_and_options(
        &module,
        &target,
        &disabled_output,
        true,
        ObjectEmissionOptions::default(),
    )
    .expect("disabled-options object should be emitted");
    let disabled_bytes = std::fs::read(&disabled_output).expect("disabled object should read");
    let default_bytes = std::fs::read(&output).expect("default object should read");
    assert_eq!(disabled_bytes, default_bytes, "disabled options must preserve default bytes");
    let disabled =
        object::File::parse(disabled_bytes.as_slice()).expect("disabled object should parse");
    assert!(
        disabled.section_by_name(".stack_sizes").is_none(),
        "disabled options must not publish .stack_sizes"
    );
    std::fs::remove_file(disabled_output).expect("disabled object should be removed");
    let non_elf_target = actus::target::TargetSpec::parse("x86_64-apple-darwin")
        .expect("Mach-O target should parse");
    let non_elf_bytes =
        actus::codegen::llvm::emit_object_bytes_with_configuration_and_options_and_float_policy(
            &module,
            &non_elf_target,
            true,
            false,
            ObjectEmissionOptions {
                emit_stack_size_section: true,
                function_sections: true,
                ..ObjectEmissionOptions::default()
            },
        )
        .expect("non-ELF object should emit");
    let non_elf =
        object::File::parse(non_elf_bytes.as_slice()).expect("non-ELF object should parse");
    assert!(
        non_elf.section_by_name(".stack_sizes").is_none(),
        "non-ELF targets must not enable the ELF frame-report option"
    );
    if std::env::var_os("ACTUS_KEEP_LLVM_OPTIONS_OBJECT").is_none() {
        std::fs::remove_file(configured_output).expect("configured object should be removed");
    } else {
        eprintln!("kept configured object at {}", configured_output.display());
    }
    std::fs::remove_file(output).expect("test object should be removed");
    std::fs::remove_file(second_output).expect("second test object should be removed");
}

#[test]
fn emits_an_arm32_thumb_object_for_the_cortex_m_profile() {
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: Vec::new(),
            }],
            cleanup: Vec::new(),
        }],
    };
    let output =
        std::env::temp_dir().join(format!("actus-llvm-arm-object-{}.o", std::process::id()));
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object(&module, &target, &output).expect("ARM object should be emitted");
    let bytes = std::fs::read(&output).expect("ARM object should be readable");
    let object = object::File::parse(bytes.as_slice()).expect("ARM object should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    assert!(object.section_by_name(".text").is_some());
    assert!(has_symbol(&object, "test__verb_main"));
    std::fs::remove_file(output).expect("ARM test object should be removed");
}

#[test]
fn emits_arm32_thumb_scalar_and_aggregate_call_fixtures() {
    let integer = BackendValueType::Integer { signed: false, width: 32 };
    let scalar_callee_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "scalar_callee").expect("scalar symbol");
    let scalar_caller_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "scalar_caller").expect("scalar symbol");
    let scalar_parameter = BackendParameter {
        name: "input".to_owned(),
        role: Role::Erg,
        ty: integer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let scalar_callee = BackendFunction {
        symbol: scalar_callee_symbol.clone(),
        hosted_entry_return: false,
        parameters: vec![scalar_parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let scalar_caller = BackendFunction {
        symbol: scalar_caller_symbol,
        hosted_entry_return: false,
        parameters: vec![scalar_parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: scalar_callee_symbol,
                    arguments: vec![BackendValueId(0)],
                    result: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };

    let aggregate_layout = actus::codegen::BackendLayout { size: 8, alignment: 4 };
    let aggregate_abi = actus::codegen::BackendDirectAggregateAbi {
        layout: aggregate_layout,
        parameter_fragments: arm_aggregate_register_fragments(),
        return_fragments: arm_aggregate_register_fragments(),
    };
    let aggregate_type = BackendValueType::Aggregate { layout: aggregate_layout };
    let aggregate_callee_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "aggregate_callee").expect("aggregate symbol");
    let aggregate_caller_symbol =
        SymbolIdentity::new("arm", SymbolKind::Verb, "aggregate_caller").expect("aggregate symbol");
    let aggregate_callee = BackendFunction {
        symbol: aggregate_callee_symbol.clone(),
        hosted_entry_return: false,
        parameters: vec![BackendParameter {
            name: "input".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate {
                abi: aggregate_abi.clone(),
            },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi: aggregate_abi.clone() },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let aggregate_caller = BackendFunction {
        symbol: aggregate_caller_symbol,
        hosted_entry_return: false,
        parameters: vec![BackendParameter {
            name: "input".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate {
                abi: aggregate_abi.clone(),
            },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi: aggregate_abi },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: aggregate_callee_symbol,
                    arguments: vec![BackendValueId(0)],
                    result: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };

    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![scalar_callee, scalar_caller, aggregate_callee, aggregate_caller],
    };
    let output =
        std::env::temp_dir().join(format!("actus-llvm-arm-call-fixtures-{}.o", std::process::id()));
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object_with_configuration_and_options(
        &module,
        &target,
        &output,
        true,
        ObjectEmissionOptions {
            emit_stack_size_section: true,
            function_sections: true,
            ..ObjectEmissionOptions::default()
        },
    )
    .expect("ARM call fixture object should be emitted");
    let bytes = std::fs::read(&output).expect("ARM call fixture object should be readable");
    let object =
        object::File::parse(bytes.as_slice()).expect("ARM call fixture object should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    assert!(
        object.section_by_name(".stack_sizes").is_some(),
        "configured ARM object must publish .stack_sizes"
    );
    let text = object.section_by_name(".text").expect("ARM call fixture should contain .text");
    assert!(text.align() >= 2, "Thumb code must have at least halfword alignment");
    let hosted_imports = ["abort", "free", "malloc", "memcpy", "printf"];
    for symbol in object.symbols().filter(|symbol| symbol.is_undefined()) {
        let name = symbol.name().expect("undefined ARM symbol should have a name");
        assert!(!hosted_imports.contains(&name), "ARM object imports hosted symbol {name}");
    }
    assert!(
        object.section_by_name(".ARM.extab").is_none(),
        "freestanding ARM object emits personality tables"
    );
    for personality in ["__aeabi_unwind_cpp_pr0", "__aeabi_unwind_cpp_pr1"] {
        assert!(
            !object
                .symbols()
                .any(|symbol| symbol.is_undefined() && symbol.name() == Ok(personality)),
            "freestanding ARM object imports {personality}"
        );
    }
    for symbol in [
        "arm__verb_scalar_5fcallee",
        "arm__verb_scalar_5fcaller",
        "arm__verb_aggregate_5fcallee",
        "arm__verb_aggregate_5fcaller",
    ] {
        assert!(object.symbols().any(|candidate| candidate.name() == Ok(symbol)));
        let section_name = format!(".text.{symbol}");
        assert!(
            object.section_by_name(&section_name).is_some(),
            "function sections must isolate `{symbol}`"
        );
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let disassembly = Command::new(llvm_tool("llvm-objdump"))
            .args(["-d", "--triple=thumbv7em-none-eabihf"])
            .arg(&output)
            .output()
            .expect("llvm-objdump should be available for ARM stack validation");
        assert!(disassembly.status.success(), "llvm-objdump failed for ARM call fixture");
        let disassembly =
            String::from_utf8(disassembly.stdout).expect("ARM disassembly should be UTF-8");
        assert_eq!(
            disassembly.matches("push	{r7, lr}").count(),
            2,
            "each ARM caller must preserve an 8-byte aligned call frame"
        );
        assert_eq!(
            disassembly.matches("pop	{r7, pc}").count(),
            2,
            "each ARM caller must restore its 8-byte aligned call frame"
        );
        assert_eq!(
            disassembly.matches("	bl	").count(),
            2,
            "each ARM caller must contain its direct call"
        );
    }
    if std::env::var_os("ACTUS_KEEP_ARM_CALL_OBJECT").is_none() {
        std::fs::remove_file(output).expect("ARM call fixture object should be removed");
    } else {
        eprintln!("kept ARM call fixture object at {}", output.display());
    }
}

#[test]
fn links_a_minimal_arm32_thumb_image_with_explicit_startup_and_provider() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-llvm-arm-image-{}", std::process::id()));
    fs::create_dir_all(&root).expect("ARM image fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(
        &manifest,
        "[package]\nname = \"arm_image\"\nversion = \"1.0.0\"\n\n[build]\ntarget = \"thumbv7em-none-eabihf\"\nlinker_script = \"memory.x\"\n",
    )
    .expect("ARM image manifest should be written");
    let startup = root.join("startup.ll");
    fs::write(
        &startup,
        r#"target triple = "thumbv7em-none-eabihf"

module asm ".syntax unified\0A.cpu cortex-m7\0A.thumb\0A.section .isr_vector,\22a\22,%progbits\0A.section .text.Reset_Handler,\22ax\22,%progbits\0A.global Reset_Handler\0A.type Reset_Handler,%function\0A.thumb_func\0AReset_Handler:\0A  bl arm__verb_boot\0A1: b 1b\0A"
"#,
    )
    .expect("ARM startup source should be written");
    let linker_script = root.join("memory.x");
    fs::write(
        &linker_script,
        "ENTRY(Reset_Handler)\nMEMORY\n{\n  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 256K\n  RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 64K\n}\n_estack = ORIGIN(RAM) + LENGTH(RAM);\nSECTIONS\n{\n  . = ORIGIN(FLASH);\n  .isr_vector ALIGN(4) : ALIGN(4) { LONG(_estack); LONG((Reset_Handler & ~1) + 1); KEEP(*(.isr_vector)) } > FLASH\n  .text : { *(.text*) *(.rodata*) } > FLASH\n  .data : { *(.data*) } > RAM AT > FLASH\n  .bss : { *(.bss*) *(COMMON) } > RAM\n  /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) }\n}\n",
    )
    .expect("ARM linker script should be written");
    let startup_object = root.join("startup.o");
    let actus_object = root.join("boot.o");
    let image = root.join("image.elf");

    run_native_command(
        Command::new(llvm_tool("llc"))
            .args(["-mtriple=thumbv7em-none-eabihf", "-mcpu=cortex-m7", "-filetype=obj"])
            .arg(&startup)
            .arg("-o")
            .arg(&startup_object),
    );

    let symbol = SymbolIdentity::new("arm", SymbolKind::Verb, "boot").expect("valid ARM symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: Vec::new(),
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object(&module, &target, &actus_object).expect("ARM Actus object should be emitted");

    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("ARM image configuration should load");
    assert!(
        !configuration.linker_is_explicit(),
        "ARM link fixtures require the bundled linker; unset ACTUS_LINKER"
    );
    link_objects(&[&startup_object, &actus_object], &image, &configuration)
        .expect("bundled ARM linker should link the image");

    let bytes = fs::read(&image).expect("ARM image should be readable");
    let object = object::File::parse(bytes.as_slice()).expect("ARM image should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    let vector = object.section_by_name(".isr_vector").expect("vector section should exist");
    let text = object.section_by_name(".text").expect("text section should exist");
    assert!(vector.align() >= 4, "vector table must be word aligned");
    assert!(text.align() >= 2, "Thumb text must be halfword aligned");
    assert!((0x0800_0000..0x0804_0000).contains(&vector.address()));
    assert!((0x0800_0000..0x0804_0000).contains(&text.address()));
    let vector_data = vector.data().expect("vector section should contain bytes");
    assert_eq!(vector_data.len(), 8, "minimal vector table must contain two words");
    let initial_stack = u32::from_le_bytes(vector_data[0..4].try_into().expect("stack word"));
    let reset_vector = u32::from_le_bytes(vector_data[4..8].try_into().expect("reset word"));
    assert_eq!(initial_stack, 0x2001_0000, "vector table must point at the linker-defined stack");
    assert_ne!(reset_vector & 1, 0, "Reset_Handler vector must select Thumb state");
    let reset_address = object
        .symbols()
        .find(|symbol| {
            symbol_name_matches(object.format(), symbol.name().unwrap_or_default(), "Reset_Handler")
        })
        .expect("Reset_Handler should be exported")
        .address();
    assert_eq!(
        u64::from(reset_vector & !1),
        reset_address & !1,
        "vector must target Reset_Handler"
    );
    assert!(has_symbol(&object, "arm__verb_boot"));
    let relocation_count =
        object.sections().map(|section| section.relocations().count()).sum::<usize>();
    assert_eq!(relocation_count, 0, "linked image must not retain relocations");
    let hosted_imports = ["abort", "free", "malloc", "memcpy", "printf"];
    for symbol in object.symbols().filter(|symbol| symbol.is_undefined()) {
        let name = symbol.name().expect("undefined symbol should have a name");
        assert!(!hosted_imports.contains(&name), "image imports hosted symbol {name}");
    }
    assert!(object.symbols().all(|symbol| !symbol.is_undefined()));
    if std::env::var_os("ACTUS_KEEP_ARM_IMAGE").is_none() {
        remove_fixture(
            &root,
            [&manifest, &startup, &linker_script, &startup_object, &actus_object, &image],
        );
    } else {
        eprintln!("kept ARM image fixture at {}", image.display());
    }
}

#[test]
fn links_an_explicit_external_c_provider_through_the_llvm_arm_path() {
    use std::process::Command;

    let root =
        std::env::temp_dir().join(format!("actus-llvm-arm-c-provider-{}", std::process::id()));
    fs::create_dir_all(&root).expect("external provider fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(
        &manifest,
        "[package]\nname = \"arm_provider\"\nversion = \"1.0.0\"\n\n[build]\ntarget = \"thumbv7em-none-eabihf\"\nlinker_script = \"memory.x\"\n",
    )
    .expect("external provider manifest should be written");
    let provider_source = root.join("provider.ll");
    fs::write(
        &provider_source,
        "target triple = \"thumbv7em-none-eabihf\"\n\ndefine i32 @runtime__extern_increment() nounwind {\n  ret i32 7\n}\n",
    )
        .expect("external C provider source should be written");
    let startup = root.join("startup.ll");
    fs::write(
        &startup,
        r#"target triple = "thumbv7em-none-eabihf"

module asm ".syntax unified\0A.cpu cortex-m7\0A.thumb\0A.section .text.Reset_Handler,\22ax\22,%progbits\0A.global Reset_Handler\0A.type Reset_Handler,%function\0A.thumb_func\0AReset_Handler:\0A  bl arm__verb_boot\0A1: b 1b\0A"
"#,
    )
    .expect("external provider startup source should be written");
    let linker_script = root.join("memory.x");
    fs::write(
        &linker_script,
        "ENTRY(Reset_Handler)\nMEMORY\n{\n  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 256K\n  RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 64K\n}\n_estack = ORIGIN(RAM) + LENGTH(RAM);\nSECTIONS\n{\n  . = ORIGIN(FLASH);\n  .isr_vector ALIGN(4) : ALIGN(4) { LONG(_estack); LONG((Reset_Handler & ~1) + 1); KEEP(*(.isr_vector)) } > FLASH\n  .text : { *(.text*) *(.rodata*) } > FLASH\n  .data : { *(.data*) } > RAM AT > FLASH\n  .bss : { *(.bss*) *(COMMON) } > RAM\n  /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) }\n}\n",
    )
    .expect("external provider linker script should be written");
    let provider_object = root.join("provider.o");
    let startup_object = root.join("startup.o");
    let actus_object = root.join("boot.o");
    let image = root.join("image.elf");

    run_native_command(
        Command::new(llvm_tool("llc"))
            .args(["-mtriple=thumbv7em-none-eabihf", "-mcpu=cortex-m7", "-filetype=obj"])
            .arg(&provider_source)
            .arg("-o")
            .arg(&provider_object),
    );
    run_native_command(
        Command::new(llvm_tool("llc"))
            .args(["-mtriple=thumbv7em-none-eabihf", "-mcpu=cortex-m7", "-filetype=obj"])
            .arg(&startup)
            .arg("-o")
            .arg(&startup_object),
    );

    let external_symbol = SymbolIdentity::new("runtime", SymbolKind::External, "increment")
        .expect("valid external symbol");
    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let module = BackendModule {
        externals: vec![BackendExternal {
            symbol: external_symbol.clone(),
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: integer,
                passing: BackendReturnMode::Direct,
            },
            runtime_pointer_return: false,
        }],
        functions: vec![BackendFunction {
            symbol: SymbolIdentity::new("arm", SymbolKind::Verb, "boot")
                .expect("valid boot symbol"),
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: integer,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: vec![
                    BackendInstruction::Call {
                        callee: external_symbol,
                        arguments: Vec::new(),
                        result: Some(BackendValueId(0)),
                        span: actus::lexer::SourceSpan::new(0, 1),
                    },
                    BackendInstruction::Return {
                        value: Some(BackendValueId(0)),
                        span: actus::lexer::SourceSpan::new(1, 2),
                    },
                ],
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::parse("thumbv7em-none-eabihf")
        .expect("Cortex-M target should parse");
    emit_object(&module, &target, &actus_object)
        .expect("Actus object with an external provider call should be emitted");

    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("external provider configuration should load");
    assert!(
        !configuration.linker_is_explicit(),
        "ARM link fixtures require the bundled linker; unset ACTUS_LINKER"
    );
    link_objects(&[&startup_object, &actus_object, &provider_object], &image, &configuration)
        .expect("bundled ARM linker should link the external provider image");

    let bytes = fs::read(&image).expect("external provider image should be readable");
    let object =
        object::File::parse(bytes.as_slice()).expect("external provider image should parse");
    assert_eq!(object.architecture(), object::Architecture::Arm);
    assert!(has_symbol(&object, "runtime__extern_increment"));
    assert!(has_symbol(&object, "arm__verb_boot"));
    let undefined = object
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .map(|symbol| symbol.name().unwrap_or("<invalid>").to_owned())
        .collect::<Vec<_>>();
    assert!(
        undefined.is_empty(),
        "linked external provider image has undefined symbols: {undefined:?}"
    );
    assert_eq!(
        object.sections().map(|section| section.relocations().count()).sum::<usize>(),
        0,
        "linked external provider image must not retain relocations"
    );
    remove_fixture(
        &root,
        [
            &manifest,
            &provider_source,
            &startup,
            &linker_script,
            &provider_object,
            &startup_object,
            &actus_object,
            &image,
        ],
    );
}

#[test]
fn executes_a_bundled_thumb_image_under_qemu() {
    let qemu = std::env::var_os("ACTUS_QEMU_SYSTEM_ARM")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("qemu-system-arm"));

    let root = std::env::temp_dir().join(format!("actus-llvm-qemu-arm-{}", std::process::id()));
    fs::create_dir_all(&root).expect("QEMU fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(
        &manifest,
        "[package]\nname = \"qemu_arm\"\nversion = \"1.0.0\"\nentry = \"boot\"\n\n[build]\ntarget = \"thumbv7em-none-eabihf\"\nlinker_script = \"memory.x\"\n",
    )
    .expect("QEMU fixture manifest should be written");
    let source = "struct Pair { erg left: u64, erg right: u64, erg extra0: u64, erg extra1: u64, erg extra2: u64, erg extra3: u64, erg extra4: u64, erg extra5: u64, } verb dividend() -> u64 { return 80u64; } verb divisor() -> u64 { return 2u64; } verb boot() -> Int { erg source: Pair = Pair { left: 1u64, right: 1u64, extra0: 0u64, extra1: 0u64, extra2: 0u64, extra3: 0u64, extra4: 0u64, extra5: 0u64, }; erg copied: Pair = source; erg numerator: u64 = dividend(); erg denominator: u64 = divisor(); erg quotient: u64 = numerator / denominator; return (copied.left + copied.right + quotient) as Int; }";
    let input = root.join("main.act");
    fs::write(&input, source).expect("QEMU Actus source should be written");
    let actus_object = root.join("actus.o");
    let build = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("build")
        .arg(&input)
        .args(["--strict", "--emit", "obj", "-o"])
        .arg(&actus_object)
        .current_dir(&root)
        .output()
        .expect("Actus CLI should start");
    assert!(
        build.status.success(),
        "Actus CLI should build the freestanding object: status={:?}, stdout={}, stderr={}",
        build.status,
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let actus_bytes = fs::read(&actus_object).expect("Actus object should be readable");
    let actus_file =
        object::File::parse(actus_bytes.as_slice()).expect("Actus object should parse");
    assert!(
        actus_file
            .symbols()
            .any(|symbol| { !symbol.is_undefined() && symbol.name().ok() == Some("boot") }),
        "Actus object must define the boot symbol called by startup"
    );

    let startup = root.join("startup.ll");
    fs::write(
        &startup,
        r#"target triple = "thumbv7em-none-eabihf"

module asm ".syntax unified\0A.cpu cortex-m7\0A.thumb\0A.section .isr_vector,\22a\22,%progbits\0A.section .text.Reset_Handler,\22ax\22,%progbits\0A.global Reset_Handler\0A.type Reset_Handler,%function\0A.thumb_func\0AReset_Handler:\0A  bl boot\0A  cmp r0, #42\0A  bne 1f\0A  ldr r1, =0x20026\0A  b 2f\0A1:\0A  ldr r1, =0x20023\0A2:\0A  movs r0, #0x18\0A  bkpt 0xab\0A3: b 3b\0A"
"#,
    )
    .expect("QEMU startup source should be written");
    let startup_object = root.join("startup.o");
    run_native_command(
        std::process::Command::new(llvm_tool("llc"))
            .args(["-mtriple=thumbv7em-none-eabihf", "-mcpu=cortex-m7", "-filetype=obj"])
            .arg(&startup)
            .arg("-o")
            .arg(&startup_object),
    );
    let linker_script = root.join("memory.x");
    fs::write(
        &linker_script,
        "ENTRY(Reset_Handler)\nMEMORY { FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 4M RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 512K }\n_estack = ORIGIN(RAM) + LENGTH(RAM);\nSECTIONS { . = ORIGIN(FLASH); .isr_vector : { LONG(_estack); LONG((Reset_Handler & ~1) + 1); KEEP(*(.isr_vector)) } > FLASH .text : { *(.text*) *(.rodata*) } > FLASH .data : { *(.data*) } > RAM AT > FLASH .bss : { *(.bss*) *(COMMON) } > RAM /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) } }\n",
    )
    .expect("QEMU linker script should be written");
    let image = root.join("image.elf");
    let configuration =
        CompilerConfiguration::from_manifest(&manifest).expect("QEMU configuration should load");
    assert!(!configuration.linker_is_explicit());
    link_objects(&[&startup_object, &actus_object], &image, &configuration)
        .expect("bundled QEMU ARM image should link");

    let bytes = fs::read(&image).expect("QEMU image should be readable");
    let object = object::File::parse(bytes.as_slice()).expect("QEMU image should parse");
    assert!(
        object.symbols().any(|symbol| {
            !symbol.is_undefined() && matches!(symbol.name(), Ok("__aeabi_uldivmod" | "__udivdi3"))
        }),
        "QEMU image must include a 64-bit division builtin"
    );
    let undefined = std::process::Command::new(llvm_tool("llvm-nm"))
        .args(["-u"])
        .arg(&image)
        .output()
        .expect("llvm-nm should start");
    assert!(undefined.status.success(), "llvm-nm failed: {undefined:?}");
    assert!(
        String::from_utf8_lossy(&undefined.stdout).trim().is_empty(),
        "bundled freestanding image has undefined runtime symbols: {}",
        String::from_utf8_lossy(&undefined.stdout)
    );
    let disassembly = std::process::Command::new(llvm_tool("llvm-objdump"))
        .args(["-d", "--triple=thumbv7em-none-eabihf", "--mattr=+fp-armv8"])
        .arg(&image)
        .output()
        .expect("llvm-objdump should start");
    assert!(disassembly.status.success(), "llvm-objdump failed: {disassembly:?}");
    let disassembly = String::from_utf8(disassembly.stdout).expect("disassembly should be UTF-8");
    for mnemonic in [
        "vadd", "vsub", "vmul", "vdiv", "vmov", "vldr", "vstr", "vpush", "vpop", "vcvt", "vcmp",
        "vmrs", "vmsr",
    ] {
        assert!(
            !disassembly.lines().any(|line| {
                let mut fields = line.split('\t');
                fields.next();
                fields
                    .next()
                    .and_then(|field| field.split_whitespace().next())
                    .is_some_and(|instruction| instruction.starts_with(mnemonic))
            }),
            "QEMU image contains forbidden VFP instruction `{mnemonic}`"
        );
    }

    if !qemu.is_file() && std::process::Command::new(&qemu).arg("--version").output().is_err() {
        if std::env::var_os("ACTUS_REQUIRE_QEMU").is_some() {
            panic!("qemu-system-arm is required for this CI job: {}", qemu.display());
        }
        eprintln!("NOT RUN: skipping QEMU ARM execution test: {} is unavailable", qemu.display());
        remove_fixture(
            &root,
            [&manifest, &input, &startup, &linker_script, &startup_object, &actus_object, &image],
        );
        return;
    }

    let mut child = std::process::Command::new(&qemu)
        .args([
            "-M",
            "mps2-an500",
            "-nographic",
            "-semihosting-config",
            "enable=on,target=native",
            "-kernel",
        ])
        .arg(&image)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("qemu-system-arm should start");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let output = loop {
        if child.try_wait().expect("QEMU status should be available").is_some() {
            break child.wait_with_output().expect("QEMU output should be readable");
        }
        if std::time::Instant::now() >= deadline {
            child.kill().expect("QEMU should be killed after timeout");
            let _ = child.wait();
            panic!("QEMU image did not exit within 30 s");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    eprintln!("QEMU exit status: {}", output.status);
    eprintln!("QEMU stdout: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("QEMU stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "QEMU image failed: {output:?}");
    remove_fixture(
        &root,
        [&manifest, &input, &startup, &linker_script, &startup_object, &actus_object, &image],
    );
}

#[cfg(target_os = "linux")]
#[test]
fn links_and_runs_a_host_object_through_the_shared_linker() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-llvm-link-{}", std::process::id()));
    fs::create_dir_all(&root).expect("link fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"llvm_link_fixture\"\nversion = \"0.1.0\"\n")
        .expect("link fixture manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern void test__verb_main(void); int main(void) { test__verb_main(); return 0; }\n",
    )
    .expect("entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "main").expect("valid symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: Vec::new(),
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: Vec::new(),
            }],
            cleanup: Vec::new(),
        }],
    };
    let llvm_object = root.join("module.o");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    emit_object(&module, &target, &llvm_object).expect("LLVM object should be emitted");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("link fixture configuration should load");
    let executable = root.join("linked");
    let objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    link_objects(&objects, &executable, &configuration).expect("host objects should link");
    let status = Command::new(&executable).status().expect("linked executable should run");
    assert_eq!(status.code(), Some(0));
    remove_fixture(&root, [&manifest, &shim_source, &shim_object, &llvm_object, &executable]);
}

#[cfg(target_os = "linux")]
#[test]
fn links_and_runs_a_repr_c_aggregate_external_provider_through_llvm() {
    use std::process::Command;

    let root =
        std::env::temp_dir().join(format!("actus-llvm-aggregate-ffi-{}", std::process::id()));
    fs::create_dir_all(&root).expect("aggregate FFI fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"llvm_aggregate_ffi_fixture\"\nversion = \"0.1.0\"\n")
        .expect("aggregate FFI manifest should be written");
    let source = "meta repr(C) struct Point { x: i32, y: i32, } unsafe extern \"C\" verb inspect(erg point: Point) -> Int; unsafe extern \"C\" verb make() -> Point; verb main() -> Int { erg point: Point = Point { x: 40i32, y: 2i32, }; erg output = make(); erg observed = inspect(point: erg output); return observed; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "aggregate FFI fixture should lex: {errors:?}");
    let program = parse(tokens).expect("aggregate FFI fixture should parse");
    let module = lower_scalar_program("llvm_aggregate_ffi", &program, Some("main"))
        .expect("aggregate FFI fixture should lower");
    let llvm_object = root.join("module.o");
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    emit_object(&module, &target, &llvm_object).expect("LLVM aggregate object should emit");

    let shim_source = root.join("provider.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nint32_t inspect(const void *raw) { const int32_t *point = raw; return point[0] + point[1]; }\nvoid make(void *raw) { int32_t *point = raw; point[0] = 40; point[1] = 2; }\n",
    )
    .expect("aggregate FFI provider should be written");
    let shim_object = root.join("provider.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("aggregate FFI configuration should load");
    let executable = root.join("aggregate-ffi");
    let objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    link_objects(&objects, &executable, &configuration)
        .expect("LLVM aggregate provider should link");
    let status =
        Command::new(&executable).status().expect("LLVM aggregate provider executable should run");
    assert_eq!(status.code(), Some(42));
    fs::remove_dir_all(&root).expect("remove aggregate FFI fixture");
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_niche_option_none_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-niche-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("niche parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"niche_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("niche parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern void *test__verb_none(void); int main(void) { return test__verb_none() == 0 ? 0 : 1; }\n",
    )
    .expect("niche parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let function = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "none").expect("valid symbol"),
        hosted_entry_return: false,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::NichePointerOption { layout, payload_layout: None },
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Option")
                        .expect("valid option symbol"),
                    discriminant: 0,
                    payload: None,
                    layout: BackendEnumLayout {
                        layout,
                        discriminant_offset: 0,
                        discriminant_size: 4,
                        payload_offset: 0,
                        payload: None,
                        niche_pointer: true,
                    },
                },
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![function] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("niche parity fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM niche object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift niche object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift niche object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM niche objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift niche objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_niche_option_some_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-niche-some-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("niche Some parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"niche_some_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("niche Some parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern void *test__verb_some(void *); int main(void) { int value = 42; return test__verb_some(&value) == &value ? 0 : 1; }\n",
    )
    .expect("niche Some parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let pointer = BackendParameter {
        name: "payload".to_owned(),
        role: Role::Erg,
        ty: BackendValueType::Pointer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let function = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "some").expect("valid symbol"),
        hosted_entry_return: false,
        parameters: vec![pointer],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: BackendValueType::NichePointerOption { layout, payload_layout: None },
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Fail {
                failure: BackendFailure {
                    result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Option")
                        .expect("valid option symbol"),
                    discriminant: 1,
                    payload: Some(BackendValueId(0)),
                    layout: BackendEnumLayout {
                        layout,
                        discriminant_offset: 0,
                        discriminant_size: 4,
                        payload_offset: 0,
                        payload: None,
                        niche_pointer: true,
                    },
                },
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![function] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("niche Some parity fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM niche Some object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift niche Some object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift niche Some object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM niche Some objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift niche Some objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_the_same_scalar_contract_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-backend-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"backend_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("parity fixture manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "extern int test__verb_entry(int); extern void test__verb_control(unsigned char); int main(void) { if (test__verb_entry(42) != 42) return 1; test__verb_control(1); return 0; }\n",
    )
    .expect("parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let integer = BackendValueType::Integer { signed: true, width: 32 };
    let identity_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "identity").expect("identity symbol");
    let entry_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "entry").expect("entry symbol");
    let control_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "control").expect("control symbol");
    let parameter = BackendParameter {
        name: "input".to_owned(),
        role: Role::Erg,
        ty: integer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let identity = BackendFunction {
        symbol: identity_symbol.clone(),
        hosted_entry_return: false,
        parameters: vec![parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let entry = BackendFunction {
        symbol: entry_symbol,
        hosted_entry_return: false,
        parameters: vec![parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: identity_symbol,
                    arguments: vec![BackendValueId(0)],
                    result: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let control = BackendFunction {
        symbol: control_symbol,
        hosted_entry_return: false,
        parameters: vec![BackendParameter {
            name: "condition".to_owned(),
            role: Role::Erg,
            ty: BackendValueType::Bool,
            passing: actus::codegen::BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Branch {
                    condition: BackendValueId(0),
                    then_block: BackendBlockId(1),
                    else_block: BackendBlockId(2),
                    then_arguments: Vec::new(),
                    else_arguments: Vec::new(),
                    span: actus::lexer::SourceSpan::new(0, 1),
                }],
            },
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(1),
                instructions: vec![BackendInstruction::Jump {
                    target: BackendBlockId(3),
                    arguments: Vec::new(),
                    span: actus::lexer::SourceSpan::new(1, 2),
                }],
            },
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(2),
                instructions: vec![BackendInstruction::Trap {
                    reason: "false parity branch".to_owned(),
                    span: actus::lexer::SourceSpan::new(2, 3),
                }],
            },
            BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(3),
                instructions: vec![BackendInstruction::Return {
                    value: None,
                    span: actus::lexer::SourceSpan::new(3, 4),
                }],
            },
        ],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![identity, entry, control] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("parity fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM parity object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift parity object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes).expect("Cranelift object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM parity objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift parity objects should link");
    let llvm_status =
        Command::new(&llvm_executable).status().expect("LLVM parity executable should run");
    let cranelift_status = Command::new(&cranelift_executable)
        .status()
        .expect("Cranelift parity executable should run");
    assert_eq!(llvm_status.code(), Some(0));
    assert_eq!(cranelift_status.code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_runtime_cleanup_order_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-cleanup-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("cleanup parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"cleanup_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("cleanup parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nstatic int order; static int status; static void *expected_buffer; static void *expected_region; static void *expected_enum; static uint64_t expected_enum_size; void actus_buffer_drop(void *value) { if (value == expected_buffer) order = order * 10 + 1; else order = -1; } int32_t actus_region_drop(void *value) { if (value == expected_region) { order = order * 10 + 2; status = 0; } else { order = -1; status = -1; } return status; } void actus_enum_drop(void *value, uint64_t size) { if (value == expected_enum && size == expected_enum_size) order = order * 10 + 3; else order = -1; } extern void test__verb_cleanup(void *, void *, void *); int main(void) { uint64_t buffer = 1; uint64_t region = 2; uint64_t enum_value[2] = { 3, 4 }; expected_buffer = &buffer; expected_region = &region; expected_enum = enum_value; expected_enum_size = sizeof(enum_value); test__verb_cleanup(&buffer, &region, enum_value); return order == 123 && status == 0 ? 0 : 1; }\n",
    )
    .expect("cleanup parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let pointer_layout = actus::codegen::BackendLayout { size: 8, alignment: 8 };
    let pointer = BackendValueType::Pointer;
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "cleanup").expect("symbol");
    let parameter = |name: &str| BackendParameter {
        name: name.to_owned(),
        role: Role::Erg,
        ty: pointer,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let buffer_descriptor = BackendCleanupResolvedDescriptor {
        descriptor: actus::codegen::BackendCleanupBridgeDescriptor {
            bridge: BackendCleanupBridge::BufferDrop,
            argument: BackendCleanupArgument::Value,
            result: BackendCleanupResult::Ignored,
        },
        layout: pointer_layout,
        field: None,
        performance_symbol: None,
    };
    let region_descriptor = BackendCleanupResolvedDescriptor {
        descriptor: actus::codegen::BackendCleanupBridgeDescriptor {
            bridge: BackendCleanupBridge::RegionDrop,
            argument: BackendCleanupArgument::Address,
            result: BackendCleanupResult::Status,
        },
        layout: pointer_layout,
        field: None,
        performance_symbol: None,
    };
    let enum_descriptor = BackendCleanupResolvedDescriptor {
        descriptor: actus::codegen::BackendCleanupBridgeDescriptor {
            bridge: BackendCleanupBridge::EnumDrop { size: 16 },
            argument: BackendCleanupArgument::Address,
            result: BackendCleanupResult::Ignored,
        },
        layout: actus::codegen::BackendLayout { size: 16, alignment: 8 },
        field: None,
        performance_symbol: None,
    };
    buffer_descriptor.validate().expect("buffer descriptor should validate");
    region_descriptor.validate().expect("region descriptor should validate");
    enum_descriptor.validate().expect("enum descriptor should validate");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: vec![parameter("buffer"), parameter("region"), parameter("enum_value")],
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: vec![
                    BackendInstruction::Cleanup {
                        action: LoweredCleanupInstruction::DropBinding {
                            binding_index: 0,
                            name: "buffer".to_owned(),
                        },
                        operand: Some(buffer_descriptor.operand(BackendValueId(0))),
                        span: actus::lexer::SourceSpan::new(0, 1),
                    },
                    BackendInstruction::Cleanup {
                        action: LoweredCleanupInstruction::DropBinding {
                            binding_index: 1,
                            name: "region".to_owned(),
                        },
                        operand: Some(region_descriptor.operand(BackendValueId(1))),
                        span: actus::lexer::SourceSpan::new(1, 2),
                    },
                    BackendInstruction::Cleanup {
                        action: LoweredCleanupInstruction::DropBinding {
                            binding_index: 2,
                            name: "enum_value".to_owned(),
                        },
                        operand: Some(enum_descriptor.operand(BackendValueId(2))),
                        span: actus::lexer::SourceSpan::new(2, 3),
                    },
                    BackendInstruction::Return {
                        value: None,
                        span: actus::lexer::SourceSpan::new(3, 4),
                    },
                ],
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("cleanup parity configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM cleanup object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift cleanup object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift cleanup object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM cleanup objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift cleanup objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_direct_register_aggregate_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-direct-aggregate-{}", std::process::id()));
    fs::create_dir_all(&root).expect("direct aggregate fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"direct_aggregate_fixture\"\nversion = \"0.1.0\"\n")
        .expect("direct aggregate manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\ntypedef struct { uint64_t first; uint64_t second; } Pair; extern Pair test__verb_caller(Pair); int main(void) { Pair input = {40, 2}; Pair output = test__verb_caller(input); return output.first == 40 && output.second == 2 ? 0 : 1; }\n",
    )
    .expect("direct aggregate entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let abi = actus::codegen::BackendDirectAggregateAbi {
        layout,
        parameter_fragments: aggregate_register_fragments(),
        return_fragments: aggregate_register_fragments(),
    };
    let aggregate_type = BackendValueType::Aggregate { layout };
    let aggregate_function = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "aggregate").expect("symbol"),
        hosted_entry_return: false,
        parameters: vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate { abi: abi.clone() },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi: abi.clone() },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let caller = BackendFunction {
        symbol: SymbolIdentity::new("test", SymbolKind::Verb, "caller").expect("symbol"),
        hosted_entry_return: false,
        parameters: vec![BackendParameter {
            name: "value".to_owned(),
            role: Role::Erg,
            ty: aggregate_type,
            passing: actus::codegen::BackendPassingMode::DirectAggregate { abi: abi.clone() },
        }],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate_type,
            passing: BackendReturnMode::DirectAggregate { abi },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: SymbolIdentity::new("test", SymbolKind::Verb, "aggregate")
                        .expect("aggregate callee symbol"),
                    arguments: vec![BackendValueId(0)],
                    result: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let module =
        BackendModule { externals: Vec::new(), functions: vec![aggregate_function, caller] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("direct aggregate fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM direct aggregate object should emit");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift direct aggregate object should emit");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift direct aggregate object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM direct aggregate objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift direct aggregate objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_result_return_slot_call_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-result-call-{}", std::process::id()));
    fs::create_dir_all(&root).expect("Result call fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"result_call_fixture\"\nversion = \"0.1.0\"\n")
        .expect("Result call manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nextern int32_t test__verb_caller(void); int main(void) { return test__verb_caller() == 0 ? 0 : 1; }\n",
    )
    .expect("Result call entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let result_layout = BackendLayout { size: 16, alignment: 8 };
    let payload_type = BackendValueType::Integer { signed: true, width: 32 };
    let result_type = BackendValueType::Enum {
        layout: BackendEnumLayout {
            layout: result_layout,
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset: 8,
            payload: Some(BackendLayout { size: 4, alignment: 4 }),
            niche_pointer: false,
        },
    };
    let callee_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "produce").expect("callee symbol");
    let caller_symbol =
        SymbolIdentity::new("test", SymbolKind::Verb, "caller").expect("caller symbol");
    let callee = BackendFunction {
        symbol: callee_symbol.clone(),
        hosted_entry_return: false,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: result_type,
            passing: BackendReturnMode::ReturnSlot { layout: result_layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Constant {
                    result: BackendValueId(0),
                    ty: payload_type,
                    value: 7,
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::EnumConstruct {
                    result: BackendValueId(1),
                    discriminant: 0,
                    payload: Some(BackendValueId(0)),
                    layout: match result_type {
                        BackendValueType::Enum { layout } => layout,
                        _ => unreachable!(),
                    },
                    payload_type: Some(payload_type),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(2, 3),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let caller = BackendFunction {
        symbol: caller_symbol,
        hosted_entry_return: false,
        parameters: Vec::new(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: payload_type,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: callee_symbol,
                    arguments: Vec::new(),
                    result: Some(BackendValueId(2)),
                    span: actus::lexer::SourceSpan::new(3, 4),
                },
                BackendInstruction::EnumDiscriminant {
                    result: BackendValueId(3),
                    subject: BackendValueId(2),
                    layout: match result_type {
                        BackendValueType::Enum { layout } => layout,
                        _ => unreachable!(),
                    },
                    span: actus::lexer::SourceSpan::new(4, 5),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(3)),
                    span: actus::lexer::SourceSpan::new(5, 6),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![callee, caller] };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("Result call fixture configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM Result call object should emit");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift Result call object should emit");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift Result call object should be written");
    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM Result call objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift Result call objects should link");
    assert_eq!(Command::new(&llvm_executable).status().unwrap().code(), Some(0));
    assert_eq!(Command::new(&cranelift_executable).status().unwrap().code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
fn aggregate_register_fragments() -> Vec<actus::codegen::BackendAggregateFragment> {
    vec![
        actus::codegen::BackendAggregateFragment {
            offset: 0,
            size: 8,
            alignment: 8,
            location: actus::codegen::BackendAggregateLocation::Register { index: 0 },
        },
        actus::codegen::BackendAggregateFragment {
            offset: 8,
            size: 8,
            alignment: 8,
            location: actus::codegen::BackendAggregateLocation::Register { index: 1 },
        },
    ]
}

fn arm_aggregate_register_fragments() -> Vec<actus::codegen::BackendAggregateFragment> {
    vec![
        actus::codegen::BackendAggregateFragment {
            offset: 0,
            size: 4,
            alignment: 4,
            location: actus::codegen::BackendAggregateLocation::Register { index: 0 },
        },
        actus::codegen::BackendAggregateFragment {
            offset: 4,
            size: 4,
            alignment: 4,
            location: actus::codegen::BackendAggregateLocation::Register { index: 1 },
        },
    ]
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_the_same_indirect_aggregate_copy_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-aggregate-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("aggregate parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"aggregate_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("aggregate parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nextern void test__verb_copy(void *, const void *); int main(void) { uint64_t source[2] = { 0x1122334455667788ULL, 0x99aabbccddeeff00ULL }; uint64_t destination[2] = { 0, 0 }; test__verb_copy(destination, source); return destination[0] == source[0] && destination[1] == source[1] ? 0 : 1; }\n",
    )
    .expect("aggregate parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "copy").expect("copy symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: vec![
                BackendParameter {
                    name: "destination".to_owned(),
                    role: Role::Ins,
                    ty: BackendValueType::Aggregate { layout },
                    passing: actus::codegen::BackendPassingMode::ExclusiveLoan { layout },
                },
                BackendParameter {
                    name: "source".to_owned(),
                    role: Role::Abs,
                    ty: BackendValueType::Aggregate { layout },
                    passing: actus::codegen::BackendPassingMode::Borrowed { layout },
                },
            ],
            return_value: BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: vec![
                    BackendInstruction::MemoryCopy {
                        destination: BackendValueId(0),
                        source: BackendValueId(1),
                        size: 16,
                        alignment: 8,
                        span: actus::lexer::SourceSpan::new(0, 1),
                    },
                    BackendInstruction::Return {
                        value: None,
                        span: actus::lexer::SourceSpan::new(1, 2),
                    },
                ],
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("aggregate parity configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM aggregate object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift aggregate object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift aggregate object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM aggregate objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift aggregate objects should link");
    let llvm_status =
        Command::new(&llvm_executable).status().expect("LLVM aggregate executable should run");
    let cranelift_status = Command::new(&cranelift_executable)
        .status()
        .expect("Cranelift aggregate executable should run");
    assert_eq!(llvm_status.code(), Some(0));
    assert_eq!(cranelift_status.code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

#[cfg(all(target_os = "linux", feature = "cranelift"))]
#[test]
fn compares_llvm_and_cranelift_on_the_same_return_slot_failure_fixture() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("actus-failure-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("failure parity fixture directory should be created");
    let manifest = root.join("Actus.toml");
    fs::write(&manifest, "[package]\nname = \"failure_parity_fixture\"\nversion = \"0.1.0\"\n")
        .expect("failure parity manifest should be written");
    let shim_source = root.join("entry.c");
    fs::write(
        &shim_source,
        "#include <stdint.h>\nextern void test__verb_fail(void *, int32_t); int main(void) { uint64_t slot[2] = { 0, 0 }; test__verb_fail(slot, 0x12345678); uint32_t *words = (uint32_t *)slot; return words[0] == 7U && words[2] == 0x12345678U ? 0 : 1; }\n",
    )
    .expect("failure parity entry shim should be written");
    let shim_object = root.join("entry.o");
    run_native_command(
        Command::new("cc").args(["-c"]).arg(&shim_source).arg("-o").arg(&shim_object),
    );

    let aggregate_layout = actus::codegen::BackendLayout { size: 16, alignment: 8 };
    let enum_layout = BackendEnumLayout {
        layout: aggregate_layout,
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset: 8,
        payload: Some(actus::codegen::BackendLayout { size: 4, alignment: 4 }),
        niche_pointer: false,
    };
    let symbol = SymbolIdentity::new("test", SymbolKind::Verb, "fail").expect("fail symbol");
    let module = BackendModule {
        externals: Vec::new(),
        functions: vec![BackendFunction {
            symbol,
            hosted_entry_return: false,
            parameters: vec![BackendParameter {
                name: "payload".to_owned(),
                role: Role::Erg,
                ty: BackendValueType::Integer { signed: true, width: 32 },
                passing: actus::codegen::BackendPassingMode::Direct,
            }],
            return_value: BackendReturn {
                role: Some(Role::Erg),
                ty: BackendValueType::Aggregate { layout: aggregate_layout },
                passing: BackendReturnMode::ReturnSlot { layout: aggregate_layout },
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                parameters: Vec::new(),
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Fail {
                    failure: BackendFailure {
                        result_type: SymbolIdentity::new("std", SymbolKind::Enum, "Result")
                            .expect("result symbol"),
                        discriminant: 7,
                        payload: Some(BackendValueId(0)),
                        layout: enum_layout,
                    },
                    span: actus::lexer::SourceSpan::new(0, 1),
                }],
            }],
            cleanup: Vec::new(),
        }],
    };
    let target = actus::target::TargetSpec::host().expect("host target should parse");
    let configuration = CompilerConfiguration::from_manifest(&manifest)
        .expect("failure parity configuration should load");
    let llvm_object = root.join("llvm.o");
    let cranelift_object = root.join("cranelift.o");
    emit_object(&module, &target, &llvm_object).expect("LLVM failure object should be emitted");
    let cranelift_bytes =
        emit_contract_module_object(&module, configuration.native_backend(), &target)
            .expect("Cranelift failure object should be emitted");
    fs::write(&cranelift_object, cranelift_bytes)
        .expect("Cranelift failure object should be written");

    let llvm_executable = root.join("llvm-linked");
    let cranelift_executable = root.join("cranelift-linked");
    let llvm_objects: [&Path; 2] = [llvm_object.as_path(), shim_object.as_path()];
    let cranelift_objects: [&Path; 2] = [cranelift_object.as_path(), shim_object.as_path()];
    link_objects(&llvm_objects, &llvm_executable, &configuration)
        .expect("LLVM failure objects should link");
    link_objects(&cranelift_objects, &cranelift_executable, &configuration)
        .expect("Cranelift failure objects should link");
    let llvm_status =
        Command::new(&llvm_executable).status().expect("LLVM failure executable should run");
    let cranelift_status = Command::new(&cranelift_executable)
        .status()
        .expect("Cranelift failure executable should run");
    assert_eq!(llvm_status.code(), Some(0));
    assert_eq!(cranelift_status.code(), Some(0));
    remove_fixture(
        &root,
        [
            &manifest,
            &shim_source,
            &shim_object,
            &llvm_object,
            &cranelift_object,
            &llvm_executable,
            &cranelift_executable,
        ],
    );
}

fn run_native_command(command: &mut std::process::Command) {
    let status = command.status().expect("native compiler should start");
    assert!(status.success(), "native compiler should succeed: {command:?}");
}

fn remove_fixture<const N: usize>(root: &Path, files: [&PathBuf; N]) {
    for file in files {
        let _ = fs::remove_file(file);
    }
    let _ = fs::remove_file(root.join("Actus.lock"));
    let _ = fs::remove_dir(root);
}

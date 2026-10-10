use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{analyze, filter_program_for_target};
use actus::target::TargetSpec;

#[cfg(unix)]
use actus::cli::run_with_args;
#[cfg(unix)]
use std::fs;

#[test]
fn filters_non_matching_target_declarations_before_semantic_analysis() {
    let source = "meta target(\"unix\") verb platform_value() -> Int { return 41; } meta target(\"windows\") verb platform_value() -> Int { return 99; } verb main() -> Int { return platform_value(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("target metadata should parse");
    let target = TargetSpec::host().expect("host target should be available");
    let filtered = filter_program_for_target(&program, &target);
    let platform_verbs = filtered
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            actus::ast::TopLevelDecl::Verb(verb) if verb.name == "platform_value" => Some(verb),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(platform_verbs.len(), 1);
    let expected = if cfg!(windows) { "windows" } else { "unix" };
    assert_eq!(
        platform_verbs[0].metadata,
        vec![actus::ast::MetaAttribute::Target(expected.into())]
    );
    analyze(&filtered).expect("filtered target program should analyze");
}

#[test]
fn target_contracts_select_platform_behavior_without_host_cfg_branches() {
    let linux = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("Linux target should parse");
    let macos = TargetSpec::parse("x86_64-apple-darwin").expect("macOS target should parse");
    let windows_msvc =
        TargetSpec::parse("x86_64-pc-windows-msvc").expect("MSVC target should parse");
    let windows_gnu =
        TargetSpec::parse("x86_64-pc-windows-gnu").expect("GNU Windows target should parse");
    let aarch64 =
        TargetSpec::parse("aarch64-unknown-linux-gnu").expect("AArch64 target should parse");
    let freestanding =
        TargetSpec::parse("x86_64-unknown-none").expect("freestanding target should parse");
    let embedded =
        TargetSpec::parse("thumbv7em-none-eabihf").expect("embedded target should parse");

    assert_eq!(linux.linker_flavor(), actus::target::LinkerFlavor::Gnu);
    assert_eq!(macos.linker_flavor(), actus::target::LinkerFlavor::Apple);
    assert_eq!(windows_msvc.linker_flavor(), actus::target::LinkerFlavor::Msvc);
    assert_eq!(windows_gnu.linker_flavor(), actus::target::LinkerFlavor::Gnu);
    assert_eq!(aarch64.pointer_width, target_lexicon::PointerWidth::U64);
    assert_eq!(aarch64.entry_contract(), actus::target::EntryContract::Hosted);
    assert_eq!(freestanding.entry_contract(), actus::target::EntryContract::Freestanding);
    assert_eq!(embedded.entry_contract(), actus::target::EntryContract::Freestanding);
    assert!(linux.matches_platform("unix"));
    assert!(macos.matches_platform("posix"));
    assert!(windows_msvc.matches_platform("windows"));
    assert!(windows_gnu.matches_platform("windows"));
    assert!(!freestanding.matches_platform("unix"));
}

#[test]
fn arm32_target_profile_declares_the_supported_thumb_mode() {
    let embedded =
        TargetSpec::parse("thumbv7em-none-eabihf").expect("embedded target should parse");
    assert_eq!(embedded.arm32_profile(), Some(actus::target::Arm32TargetProfile::CortexMv7em));
    assert_eq!(embedded.arm_instruction_mode(), Some(actus::target::ArmInstructionMode::Thumb2));
    let abi = embedded.arm32_abi_contract().expect("ARM ABI should be defined");
    assert_eq!(abi.word_bytes, 4);
    assert_eq!(abi.pointer_width_bits, 32);
    assert_eq!(abi.stack_alignment_bytes, 8);
    assert_eq!(abi.argument_registers, [0, 1, 2, 3]);
    assert_eq!(abi.return_registers, [0, 1]);
    assert_eq!(abi.callee_saved_registers, [4, 5, 6, 7, 8, 9, 10, 11]);
    assert_eq!(
        actus::target::Arm32AbiContract::scalar_word_representation(),
        actus::target::Arm32ValueRepresentation {
            size_bytes: 4,
            alignment_bytes: 4,
            argument: actus::target::Arm32ArgumentLocation::CoreRegisters(
                actus::target::Arm32RegisterWords { first_register: 0, word_count: 1 },
            ),
            return_value: actus::target::Arm32ReturnLocation::CoreRegisters(
                actus::target::Arm32RegisterWords { first_register: 0, word_count: 1 },
            ),
        }
    );
    assert_eq!(
        actus::target::Arm32AbiContract::aggregate_stack_representation(12, 4, 16),
        actus::target::Arm32ValueRepresentation {
            size_bytes: 12,
            alignment_bytes: 4,
            argument: actus::target::Arm32ArgumentLocation::Stack {
                byte_offset: 16,
                byte_size: 12,
                alignment_bytes: 4,
            },
            return_value: actus::target::Arm32ReturnLocation::IndirectCoreRegister {
                address_register: 0,
            },
        }
    );

    let hosted = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("host target should parse");
    assert_eq!(hosted.arm32_profile(), None);
    assert_eq!(hosted.arm_instruction_mode(), None);
    assert_eq!(hosted.arm32_abi_contract(), None);
}

#[test]
fn artifact_contract_records_startup_and_linker_requirements() {
    let hosted = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
    let freestanding = TargetSpec::parse("thumbv7em-none-eabihf").expect("target should parse");

    assert_eq!(hosted.artifact_contract().startup, actus::target::StartupContract::HostedRuntime);
    assert_eq!(
        freestanding.artifact_contract().startup,
        actus::target::StartupContract::ExplicitEntry
    );
    assert_eq!(hosted.artifact_contract().linker_flavor, actus::target::LinkerFlavor::Gnu);
    assert_eq!(
        hosted.artifact_contract().entry_visibility,
        actus::target::SymbolVisibility::Exported
    );
    assert_eq!(
        hosted.artifact_contract().runtime_visibility,
        actus::target::SymbolVisibility::Imported
    );
}

#[test]
fn target_contracts_expose_supported_pointer_widths_and_byte_orders() {
    let little_32 =
        TargetSpec::parse("i686-unknown-linux-gnu").expect("32-bit target should parse");
    let big_32 =
        TargetSpec::parse("powerpc-unknown-linux-gnu").expect("big-endian target should parse");
    let big_64 = TargetSpec::parse("powerpc64-unknown-linux-gnu")
        .expect("64-bit big-endian target should parse");

    assert_eq!(little_32.pointer_width, target_lexicon::PointerWidth::U32);
    assert_eq!(little_32.endianness, target_lexicon::Endianness::Little);
    assert_eq!(big_32.pointer_width, target_lexicon::PointerWidth::U32);
    assert_eq!(big_32.endianness, target_lexicon::Endianness::Big);
    assert_eq!(big_64.pointer_width, target_lexicon::PointerWidth::U64);
    assert_eq!(big_64.endianness, target_lexicon::Endianness::Big);
}

#[test]
fn hosted_provider_contract_is_explicit_and_cache_stable() {
    let provider = actus::target::TimeProviderContract::hosted_default();
    assert_eq!(provider.clock_unit(), actus::target::ClockUnit::Nanoseconds);
    assert_eq!(provider.counter_width(), 64);
    assert_eq!(provider.frequency_hz(), 1_000_000_000);
    assert_eq!(provider.read_symbol(), "actus_monotonic_nanos");
    assert_eq!(provider.interrupt_safety(), actus::target::InterruptSafety::Safe);
    assert_eq!(provider.sleep(), actus::target::SleepPolicy::Continues);
    assert_eq!(provider.ticks_to_nanos(7), Ok(7));
    assert_eq!(provider.spec_hash("target"), provider.spec_hash("target"));
}

#[cfg(unix)]
#[test]
fn native_build_ignores_non_matching_target_symbols() {
    let root = std::env::temp_dir().join(format!("actus-meta-target-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = "meta target(\"unix\") verb platform_value() -> Int { return 41; } meta target(\"windows\") verb platform_value() -> Int { return 99; } verb main() -> Int { return platform_value(); }";
    fs::write(&input, source).expect("write target fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run target fixture");
    assert_eq!(status.code(), Some(41));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

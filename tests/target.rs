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
    let freestanding =
        TargetSpec::parse("x86_64-unknown-none").expect("freestanding target should parse");
    let embedded =
        TargetSpec::parse("thumbv7em-none-eabihf").expect("embedded target should parse");

    assert_eq!(linux.linker_flavor(), actus::target::LinkerFlavor::Gnu);
    assert_eq!(macos.linker_flavor(), actus::target::LinkerFlavor::Apple);
    assert_eq!(windows_msvc.linker_flavor(), actus::target::LinkerFlavor::Msvc);
    assert_eq!(windows_gnu.linker_flavor(), actus::target::LinkerFlavor::Gnu);
    assert_eq!(freestanding.entry_contract(), actus::target::EntryContract::Freestanding);
    assert_eq!(embedded.entry_contract(), actus::target::EntryContract::Freestanding);
    assert!(linux.matches_platform("unix"));
    assert!(macos.matches_platform("posix"));
    assert!(windows_msvc.matches_platform("windows"));
    assert!(windows_gnu.matches_platform("windows"));
    assert!(!freestanding.matches_platform("unix"));
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

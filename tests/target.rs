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

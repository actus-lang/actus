use actus::codegen::{SymbolError, SymbolIdentity, SymbolKind, SymbolRegistry};
use actus::configuration::NativeBackendConfiguration;
use actus::lexer::scan;
use actus::parser::parse;
use actus::target::TargetSpec;
use object::{Object, ObjectSymbol};

#[test]
fn symbol_identity_is_stable_across_declaration_families() {
    let verb = SymbolIdentity::new("actus_mod_3_std_2_fs", SymbolKind::Verb, "read").unwrap();
    let data = SymbolIdentity::new("actus_mod_3_std_2_fs", SymbolKind::Data, "read").unwrap();
    assert_eq!(verb.as_str(), "actus_mod_3_std_2_fs__verb_read");
    assert_ne!(verb, data);
}

#[test]
fn symbol_identity_escapes_specialization_without_ambiguity() {
    let first = SymbolIdentity::specialized(
        "actus_root",
        SymbolKind::Generic,
        "Box[Int]",
        &["Reader", "File"],
    )
    .unwrap();
    let second = SymbolIdentity::specialized(
        "actus_root",
        SymbolKind::Generic,
        "Box_BInt",
        &["Reader", "File"],
    )
    .unwrap();
    assert_ne!(first, second);
    assert_eq!(first.as_str(), "actus_root__generic_Box_5bInt_5d__Reader_File");
}

#[test]
fn registry_rejects_only_exact_identity_duplicates() {
    let mut registry = SymbolRegistry::default();
    let first = SymbolIdentity::new("actus_root", SymbolKind::Verb, "run").unwrap();
    registry.register(first.clone()).unwrap();
    registry.register(SymbolIdentity::new("actus_root", SymbolKind::Data, "run").unwrap()).unwrap();
    assert_eq!(
        registry.register(first),
        Err(SymbolError::Duplicate("actus_root__verb_run".to_owned()))
    );
}

#[test]
fn identity_rejects_missing_namespace_and_name() {
    assert_eq!(SymbolIdentity::new("", SymbolKind::Verb, "main"), Err(SymbolError::EmptyNamespace));
    assert_eq!(
        SymbolIdentity::new("actus_root", SymbolKind::Verb, ""),
        Err(SymbolError::EmptyName)
    );
}

#[test]
fn native_objects_follow_namespace_context_deterministically() {
    let source = "verb helper() -> Int { return 1; } verb main() -> Int { return helper(); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();
    let target = TargetSpec::host().unwrap();
    let configuration = NativeBackendConfiguration::default();
    let first = actus::codegen::emit_program_object_for_target_in_namespace(
        &program,
        "main",
        "actus_mod_4_left",
        &configuration,
        &target,
    )
    .unwrap();
    let repeat = actus::codegen::emit_program_object_for_target_in_namespace(
        &program,
        "main",
        "actus_mod_4_left",
        &configuration,
        &target,
    )
    .unwrap();
    let other = actus::codegen::emit_program_object_for_target_in_namespace(
        &program,
        "main",
        "actus_mod_5_right",
        &configuration,
        &target,
    )
    .unwrap();
    assert_eq!(first, repeat);
    assert_ne!(first, other);
}

#[test]
fn imported_module_objects_emit_without_an_entry_and_preserve_public_wrappers() {
    let source = "verb hidden() -> Int { return 41; } verb unused() -> Int { return 99; } open verb add() -> Int { return hidden() + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();
    let bytes = actus::codegen::emit_module_object_for_target_in_namespace(
        &program,
        "actus_mod_4_math",
        &NativeBackendConfiguration::default(),
        &TargetSpec::host().unwrap(),
    )
    .unwrap();
    let file = object::File::parse(bytes.as_slice()).unwrap();
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(symbols.iter().any(|symbol| { symbol_matches(symbol, "actus_mod_4_math__verb_add") }));
    assert!(
        symbols.iter().any(|symbol| { symbol_matches(symbol, "actus_mod_4_math__verb_hidden") })
    );
    assert!(
        !symbols.iter().any(|symbol| { symbol_matches(symbol, "actus_mod_4_math__verb_unused") })
    );
}

#[test]
fn emits_each_reachable_generic_instance_once_in_its_namespace() {
    let source = "verb read[N: Usize]() -> u32 { return N as u32; } verb main() -> Int { return read[4]() as Int + read[8]() as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();
    let bytes = actus::codegen::emit_program_object_for_target_in_namespace(
        &program,
        "main",
        "actus_mod_generic",
        &NativeBackendConfiguration::default(),
        &TargetSpec::host().unwrap(),
    )
    .unwrap();
    let file = object::File::parse(bytes.as_slice()).unwrap();
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert_eq!(symbols.iter().filter(|symbol| symbol.contains("__verb_read_5f_5f4")).count(), 1);
    assert_eq!(symbols.iter().filter(|symbol| symbol.contains("__verb_read_5f_5f8")).count(), 1);
}

#[test]
fn generic_specializations_are_scoped_by_module_namespace() {
    let source = "verb read[N: Usize]() -> u32 { return N as u32; } verb main() -> Int { return read[4]() as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();
    let target = TargetSpec::host().unwrap();
    let configuration = NativeBackendConfiguration::default();
    let left = actus::codegen::emit_program_object_for_target_in_namespace(
        &program,
        "main",
        "actus_mod_4_left",
        &configuration,
        &target,
    )
    .unwrap();
    let right = actus::codegen::emit_program_object_for_target_in_namespace(
        &program,
        "main",
        "actus_mod_5_right",
        &configuration,
        &target,
    )
    .unwrap();
    let left_file = object::File::parse(left.as_slice()).unwrap();
    let right_file = object::File::parse(right.as_slice()).unwrap();
    let left_symbols =
        left_file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    let right_symbols =
        right_file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert!(left_symbols.iter().any(|symbol| symbol.contains("actus_mod_4_left")));
    assert!(right_symbols.iter().any(|symbol| symbol.contains("actus_mod_5_right")));
    assert!(!left_symbols.iter().any(|symbol| symbol.contains("actus_mod_5_right")));
    assert!(!right_symbols.iter().any(|symbol| symbol.contains("actus_mod_4_left")));
}

#[test]
fn failed_specialization_does_not_poison_a_later_emission() {
    let invalid = "verb read[N: Usize]() -> u32 { return N as u32; } verb main() -> Int { return missing(); }";
    let (tokens, errors) = scan(invalid);
    assert!(errors.is_empty());
    let invalid_program = parse(tokens).unwrap();
    assert!(
        actus::codegen::emit_program_object_for_target_in_namespace(
            &invalid_program,
            "main",
            "actus_mod_retry",
            &NativeBackendConfiguration::default(),
            &TargetSpec::host().unwrap(),
        )
        .is_err()
    );

    let valid = "verb read[N: Usize]() -> u32 { return N as u32; } verb main() -> Int { return read[4]() as Int; }";
    let (tokens, errors) = scan(valid);
    assert!(errors.is_empty());
    let valid_program = parse(tokens).unwrap();
    let bytes = actus::codegen::emit_program_object_for_target_in_namespace(
        &valid_program,
        "main",
        "actus_mod_retry",
        &NativeBackendConfiguration::default(),
        &TargetSpec::host().unwrap(),
    )
    .expect("a failed emission must not poison a later valid emission");
    let file = object::File::parse(bytes.as_slice()).unwrap();
    let symbols = file.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
    assert_eq!(symbols.iter().filter(|symbol| symbol.contains("__verb_read_5f_5f4")).count(), 1);
}

fn symbol_matches(actual: &str, expected: &str) -> bool {
    actual == expected || actual.strip_prefix('_') == Some(expected)
}

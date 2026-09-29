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
    let source =
        "verb hidden() -> Int { return 41; } open verb add() -> Int { return hidden() + 1; }";
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
}

fn symbol_matches(actual: &str, expected: &str) -> bool {
    actual == expected || actual.strip_prefix('_') == Some(expected)
}

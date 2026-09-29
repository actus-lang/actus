use actus::codegen::{SymbolError, SymbolIdentity, SymbolKind, SymbolRegistry};

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

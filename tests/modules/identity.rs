use actus::modules::ModuleNamespace;

#[test]
fn root_namespace_is_reserved_and_stable() {
    let namespace = ModuleNamespace::root();
    assert_eq!(namespace.module_path(), "<root>");
    assert_eq!(namespace.symbol_prefix(), "actus_root");
}

#[test]
fn module_identity_is_shared_by_unit_and_symbol_layers() {
    let namespace = ModuleNamespace::from_module_path("std::fs").unwrap();
    assert_eq!(namespace.module_path(), "std::fs");
    assert_eq!(namespace.symbol_prefix(), "actus_mod_3_std_2_fs");
}

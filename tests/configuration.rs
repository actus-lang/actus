use std::fs;

use actus::configuration::CompilerConfiguration;

#[test]
fn loads_build_settings_from_an_arca_manifest() {
    let path = std::env::temp_dir().join(format!("actus-config-{}.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.2.3\"\nentry = \"main\"\n\n[build]\nlinker = \"clang\"\nnative_module = \"sample_native\"\nposition_independent = false\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest(&path).expect("manifest should load");
    assert_eq!(configuration.linker().to_string_lossy(), "clang");
    assert_eq!(configuration.entry_symbol(), Some("main"));
    assert_eq!(configuration.native_backend().module_name(), "sample_native");
    assert!(!configuration.native_backend().position_independent());
    let _ = fs::remove_file(path);
}

#[test]
fn rejects_unknown_manifest_fields() {
    let path =
        std::env::temp_dir().join(format!("actus-config-invalid-{}.toml", std::process::id()));
    fs::write(&path, "[package]\nname = \"sample\"\nversion = \"1.2.3\"\nunknown = true\n")
        .expect("write manifest");

    let error = CompilerConfiguration::from_manifest(&path).expect_err("unknown field must fail");
    assert!(error.to_string().contains("unknown field"));
    let _ = fs::remove_file(path);
}

#[test]
fn rejects_empty_native_module_names() {
    let path = std::env::temp_dir().join(format!("actus-config-empty-{}.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.2.3\"\n\n[build]\nnative_module = \"  \"\n",
    )
    .expect("write manifest");

    let error = CompilerConfiguration::from_manifest(&path).expect_err("empty module must fail");
    assert!(error.to_string().contains("native_module must not be empty"));
    let _ = fs::remove_file(path);
}

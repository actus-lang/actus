use std::fs;

use actus::configuration::{BuildProfile, CompilerConfiguration, LibraryKind, OptimizationLevel};
use actus::diagnostics::{DiagnosticSeverity, STRICT_LEGACY_DEPENDENCY, STRICT_LEGACY_MANIFEST};
use actus::target::TargetSpec;

#[test]
fn loads_build_settings_from_an_actus_manifest() {
    let path = std::env::temp_dir().join(format!("actus-config-{}.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.2.3\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nlinker = \"clang\"\nnative_module = \"sample_native\"\nposition_independent = false\n\n[[build.libraries]]\nname = \"m\"\nkind = \"shared\"\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest(&path).expect("manifest should load");
    assert_eq!(configuration.linker().to_string_lossy(), "clang");
    assert_eq!(configuration.entry_symbol(), Some("main"));
    assert_eq!(configuration.native_backend().module_name(), "sample_native");
    assert!(!configuration.native_backend().position_independent());
    assert_eq!(configuration.libraries()[0].name(), "m");
    assert_eq!(configuration.libraries()[0].kind(), LibraryKind::Shared);
    assert!(configuration.library_paths().is_empty());
    let expected_linker = TargetSpec::host().expect("host target should parse").linker_flavor();
    assert_eq!(configuration.linker_flavor(), expected_linker);
    let _ = fs::remove_file(path);
}

#[test]
fn source_limits_are_enabled_when_package_policy_is_absent() {
    let root =
        std::env::temp_dir().join(format!("actus-config-source-limits-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(&path, "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n")
        .expect("write manifest");

    let configuration =
        CompilerConfiguration::from_manifest_read_only(&path).expect("manifest should load");
    assert_eq!(configuration.source_limit_mode(), actus::configuration::SourceLimitMode::Enabled);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn accepts_package_wide_limitless_source_policy() {
    let root = std::env::temp_dir().join(format!("actus-config-limitless-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\nsource_limits = \"limitless\"\n",
    )
    .expect("write manifest");

    let configuration =
        CompilerConfiguration::from_manifest_read_only(&path).expect("manifest should load");
    assert_eq!(configuration.source_limit_mode(), actus::configuration::SourceLimitMode::Limitless);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn rejects_unknown_source_limit_policy() {
    let root =
        std::env::temp_dir().join(format!("actus-config-invalid-limits-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\nsource_limits = \"off\"\n",
    )
    .expect("write manifest");

    let error = CompilerConfiguration::from_manifest_read_only(&path)
        .expect_err("unknown source-limit policy must fail");
    assert!(error.to_string().contains("source_limits"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn loads_target_profile_and_hash_for_capsula_artifacts() {
    let root = std::env::temp_dir().join(format!("actus-target-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create project directory");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n\n[build]\ntarget = \"x86_64-unknown-linux-gnu\"\nprofile = \"release\"\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest(&path).expect("manifest should load");
    let target = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
    assert_eq!(configuration.target().triple(), target.triple());
    assert_eq!(configuration.target_spec_hash(), target.spec_hash());
    assert_eq!(configuration.profile(), BuildProfile::Release);
    assert_eq!(configuration.entry_contract(), actus::configuration::EntryContract::Hosted);
    assert_eq!(
        configuration.capsula_target_directory(),
        root.join("capsula/release/x86_64-unknown-linux-gnu")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn excludes_host_runtime_for_freestanding_targets() {
    let root = std::env::temp_dir().join(format!("actus-freestanding-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create project directory");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"bare\"\nversion = \"0.1.0\"\n\n[build]\ntarget = \"x86_64-unknown-none\"\n",
    )
    .expect("write manifest");

    let configuration =
        CompilerConfiguration::from_manifest(&path).expect("freestanding manifest should load");
    assert_eq!(configuration.entry_contract(), actus::configuration::EntryContract::Freestanding);
    assert!(!configuration.host_runtime_enabled());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn loads_profile_tables_and_maps_release_to_speed_optimization() {
    let path = std::env::temp_dir().join(format!("actus-profiles-{}.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n\n[profile.debug]\nopt_level = \"none\"\n\n[profile.release]\nopt_level = \"speed\"\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest(&path).expect("manifest should load");
    assert_eq!(configuration.profile(), BuildProfile::Debug);
    assert_eq!(configuration.native_backend().optimization_level(), OptimizationLevel::None);
    let release = configuration.with_profile(BuildProfile::Release);
    assert_eq!(release.profile(), BuildProfile::Release);
    assert_eq!(release.native_backend().optimization_level(), OptimizationLevel::Speed);
    let _ = fs::remove_file(path);
}

#[test]
fn derives_target_linker_unless_actus_overrides_it() {
    let root = std::env::temp_dir().join(format!("actus-linker-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create project directory");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n\n[build]\ntarget = \"x86_64-pc-windows-msvc\"\nlinker = \"custom-linker\"\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest(&path).expect("manifest should load");
    assert_eq!(configuration.linker().to_string_lossy(), "custom-linker");
    assert_eq!(configuration.entry_contract(), actus::configuration::EntryContract::Hosted);
    let _ = fs::remove_dir_all(root);
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

#[test]
fn rejects_unsafe_library_names() {
    let path = std::env::temp_dir()
        .join(format!("actus-config-library-{}-invalid.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.2.3\"\n\n[[build.libraries]]\nname = \"-lmalicious\"\nkind = \"static\"\n",
    )
    .expect("write manifest");

    let error = CompilerConfiguration::from_manifest(&path).expect_err("unsafe name must fail");
    assert!(error.to_string().contains("must not start with `-`"));
    let _ = fs::remove_file(path);
}

#[test]
fn rejects_unknown_language_editions() {
    let path =
        std::env::temp_dir().join(format!("actus-config-edition-{}.toml", std::process::id()));
    fs::write(&path, "[package]\nname = \"sample\"\nversion = \"1.2.3\"\nedition = \"future\"\n")
        .expect("write manifest");

    let error = CompilerConfiguration::from_manifest(&path).expect_err("unknown edition must fail");
    assert!(error.to_string().contains("unsupported Actus edition"));
    let _ = fs::remove_file(path);
}

#[test]
fn strict_configuration_rejects_legacy_manifest_warnings() {
    let root = std::env::temp_dir().join(format!("actus-strict-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create strict configuration directory");
    let path = root.join("Arca.toml");
    fs::write(&path, "[package]\nname = \"legacy\"\nversion = \"1.0.0\"\n")
        .expect("write legacy manifest");

    let error = CompilerConfiguration::from_manifest_strict(&path)
        .expect_err("strict mode must reject legacy manifest");
    assert_eq!(error.diagnostic().code(), STRICT_LEGACY_MANIFEST);
    assert_eq!(error.diagnostic().severity(), DiagnosticSeverity::Error);
    assert_eq!(error.diagnostic().source_path(), Some(path.to_str().unwrap()));
    assert!(error.to_string().contains("E1801"));
    assert!(error.to_string().contains("strict mode rejects deprecated `Arca.toml`"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn strict_configuration_codes_legacy_dependency_manifest_failures() {
    let root = std::env::temp_dir().join(format!("actus-strict-dependency-{}", std::process::id()));
    let dependency = root.join("legacy");
    fs::create_dir_all(dependency.join("src")).expect("create dependency source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"root\"\nversion = \"1.0.0\"\n\n[dependencies.legacy]\npath = \"legacy\"\n",
    )
    .expect("write root manifest");
    fs::write(dependency.join("Arca.toml"), "[package]\nname = \"legacy\"\nversion = \"1.0.0\"\n")
        .expect("write legacy dependency manifest");

    let error = CompilerConfiguration::from_manifest_strict(&root.join("Actus.toml"))
        .expect_err("strict mode must reject a legacy dependency manifest");
    assert_eq!(error.diagnostic().code(), STRICT_LEGACY_DEPENDENCY);
    assert_eq!(error.diagnostic().severity(), DiagnosticSeverity::Error);
    let _ = fs::remove_dir_all(root);
}

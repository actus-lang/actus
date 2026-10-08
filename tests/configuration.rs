use std::fs;

use actus::configuration::{
    BuildProfile, CompilerConfiguration, LibraryKind, OptimizationLevel, RuntimeProfile,
};
use actus::diagnostics::{
    DiagnosticSeverity, STRICT_LEGACY_DEPENDENCY, STRICT_LEGACY_MANIFEST,
    STRICT_RUNTIME_TARGET_INCOMPATIBLE, STRICT_RUNTIME_UNSUPPORTED,
};
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
    assert_eq!(configuration.runtime_profile(), RuntimeProfile::Core);
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
fn loads_package_zero_float_native_policy() {
    let path =
        std::env::temp_dir().join(format!("actus-config-no-float-{}.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"systems\"\nversion = \"1.0.0\"\n\n[build]\nverify_no_float_ir = true\n",
    )
    .expect("write manifest");

    let configuration =
        CompilerConfiguration::from_manifest(&path).expect("zero-float package policy should load");
    assert!(configuration.native_backend().verifies_no_float_ir());
    let _ = fs::remove_file(path);
}

#[test]
fn loads_and_hashes_an_explicit_manifest_time_provider() {
    let path =
        std::env::temp_dir().join(format!("actus-time-provider-{}.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n\n[build.time_provider]\nname = \"fake-counter\"\nread_symbol = \"fake_read\"\nclock_unit = \"ticks\"\ncounter_width = 32\nwrap_behavior = \"extendmodulo\"\nfrequency_hz = 1000000\nread_atomicity = \"singleword\"\ninterrupt_safety = \"safe\"\ninitialization = \"readyatentry\"\ncalibration = \"fixed\"\nsleep = \"continues\"\nreset = \"resets\"\ndiscontinuity = \"reject\"\n",
    )
    .expect("write provider manifest");

    let configuration = CompilerConfiguration::from_manifest(&path)
        .expect("explicit provider manifest should load");
    let provider = configuration.time_provider().expect("provider should be configured");
    assert_eq!(provider.name(), "fake-counter");
    assert_eq!(provider.counter_width(), 32);
    assert_ne!(configuration.target_spec_hash(), TargetSpec::host().unwrap().spec_hash());
    let _ = fs::remove_file(path);
}

#[test]
fn rejects_ambiguous_time_provider_contracts() {
    let path = std::env::temp_dir()
        .join(format!("actus-time-provider-invalid-{}.toml", std::process::id()));
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n\n[build.time_provider]\nname = \"fake-counter\"\nread_symbol = \"fake_read\"\nclock_unit = \"ticks\"\ncounter_width = 32\nwrap_behavior = \"extendmodulo\"\nfrequency_hz = 0\nread_atomicity = \"singleword\"\ninterrupt_safety = \"safe\"\ninitialization = \"readyatentry\"\ncalibration = \"fixed\"\nsleep = \"continues\"\nreset = \"resets\"\ndiscontinuity = \"reject\"\n",
    )
    .expect("write invalid provider manifest");

    let error = CompilerConfiguration::from_manifest(&path)
        .expect_err("zero provider frequency must be rejected");
    assert!(error.to_string().contains("frequency_hz must be positive"));
    let _ = fs::remove_file(path);
}

#[test]
fn loads_explicit_standard_runtime_profile() {
    let root = std::env::temp_dir().join(format!("actus-runtime-std-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest_read_only(&path)
        .expect("standard runtime manifest should load");
    assert_eq!(configuration.runtime_profile(), RuntimeProfile::Std);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn allows_standard_runtime_profile_for_freestanding_target() {
    let root = std::env::temp_dir().join(format!("actus-runtime-mismatch-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"bare\"\nversion = \"1.0.0\"\n\n[build]\nruntime = \"std\"\ntarget = \"x86_64-unknown-none\"\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest_read_only(&path)
        .expect("target-aware std must load for freestanding targets");
    assert_eq!(configuration.runtime_profile(), RuntimeProfile::Std);
    assert_eq!(configuration.runtime_module_roots().len(), 2);
    assert!(configuration.runtime_module_roots().contains_key("region"));
    assert!(configuration.runtime_module_roots().contains_key("wire"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn allows_standard_runtime_profile_for_embedded_target_fixture() {
    let root = std::env::temp_dir().join(format!("actus-runtime-embedded-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"embedded\"\nversion = \"1.0.0\"\n\n[build]\nruntime = \"std\"\ntarget = \"thumbv7em-none-eabihf\"\n",
    )
    .expect("write manifest");

    let configuration = CompilerConfiguration::from_manifest_read_only(&path)
        .expect("embedded std manifest should load");
    assert_eq!(configuration.runtime_profile(), RuntimeProfile::Std);
    assert!(!configuration.host_runtime_enabled());
    assert_eq!(configuration.runtime_module_roots().len(), 2);
    assert!(configuration.runtime_module_roots().contains_key("region"));
    assert!(configuration.runtime_module_roots().contains_key("wire"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn rejects_freestanding_runtime_profile_for_hosted_target() {
    let root =
        std::env::temp_dir().join(format!("actus-runtime-hosted-mismatch-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"hosted\"\nversion = \"1.0.0\"\n\n[build]\nruntime = \"freestanding\"\n",
    )
    .expect("write manifest");

    let error = CompilerConfiguration::from_manifest_strict(&path)
        .expect_err("freestanding runtime must be rejected for hosted targets");
    assert_eq!(error.diagnostic().code(), STRICT_RUNTIME_TARGET_INCOMPATIBLE);
    assert!(error.to_string().contains("incompatible with hosted target"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn rejects_unknown_runtime_profile_with_a_stable_diagnostic() {
    let root = std::env::temp_dir().join(format!("actus-runtime-unknown-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create source root");
    let path = root.join("Actus.toml");
    fs::write(
        &path,
        "[package]\nname = \"unknown\"\nversion = \"1.0.0\"\n\n[build]\nruntime = \"portable\"\n",
    )
    .expect("write manifest");

    let error = CompilerConfiguration::from_manifest_strict(&path)
        .expect_err("unknown runtime profile must be rejected");
    assert_eq!(error.diagnostic().code(), STRICT_RUNTIME_UNSUPPORTED);
    assert!(error.to_string().contains("unsupported runtime profile"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn rejects_dependency_alias_that_shadows_builtin_standard_library() {
    let root =
        std::env::temp_dir().join(format!("actus-runtime-alias-conflict-{}", std::process::id()));
    let dependency = root.join("local_std");
    fs::create_dir_all(dependency.join("src")).expect("create dependency source root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"sample\"\nversion = \"1.0.0\"\n\n[build]\nruntime = \"std\"\n\n[dependencies.std]\npath = \"local_std\"\n",
    )
    .expect("write root manifest");
    fs::write(
        dependency.join("Actus.toml"),
        "[package]\nname = \"local_std\"\nversion = \"1.0.0\"\n",
    )
    .expect("write dependency manifest");

    let error = CompilerConfiguration::from_manifest_read_only(&root.join("Actus.toml"))
        .expect_err("builtin std alias must not be shadowed");
    assert!(error.to_string().contains("conflicts with the builtin standard library"));
    fs::remove_dir_all(root).expect("remove fixture");
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

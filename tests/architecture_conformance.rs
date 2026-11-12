use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_FILENAMES: &[&str] = &[
    "utils.rs",
    "helpers.rs",
    "common.rs",
    "misc.rs",
    "support.rs",
    "implementation.rs",
    "entry.rs",
    "core.rs",
    "base.rs",
    "impl.rs",
    "stuff.rs",
    "utils.act",
    "helpers.act",
    "common.act",
    "misc.act",
];

const RUST_KEYWORD_FILENAMES: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while", "yield",
];

const ALLOWED_NON_FACADE_MOD_RS: &[&str] =
    &["src/diagnostics/renderer/mod.rs", "src/formatter/mod.rs"];

const ALLOWED_RS_PAIRS: &[&str] = &[];

const ALLOWED_SINGLE_FILE_MODULES: &[&str] = &[];

const ALLOWED_PARENT_NAME_DUPLICATES: &[&str] = &[];

const ALLOWED_FORBIDDEN_FILES: &[&str] = &["src/semantic/enum_constructors/support.rs"];

const ALLOWED_PATH_MODULES: &[&str] = &[
    "src/conformance/source_limits.rs",
    "src/diagnostics/renderer/codes/mod.rs",
    "src/ffi/abi.rs",
];

fn source_relative(path: &Path) -> String {
    path.strip_prefix(repository_path(".")).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn child_directories(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .expect("architecture fixture directory should be readable")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            path.is_dir().then_some(path)
        })
        .collect()
}

fn has_direct_act_file(directory: &Path) -> bool {
    fs::read_dir(directory)
        .expect("Actus source directory should be readable")
        .filter_map(Result::ok)
        .any(|entry| {
            entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "act")
        })
}

fn assert_canonical_facades(directory: &Path) {
    for child in child_directories(directory) {
        if has_direct_act_file(&child) {
            let name = child.file_name().expect("module directory name").to_string_lossy();
            assert!(
                child.join(format!("{name}.act")).is_file(),
                "module directory {} must expose {}.act",
                child.display(),
                name
            );
        }
        assert_canonical_facades(&child);
    }
}

fn assert_no_forbidden_filenames(directory: &Path) {
    for entry in fs::read_dir(directory).expect("source directory should be readable") {
        let path = entry.expect("source directory entry").path();
        if path.is_dir() {
            assert_no_forbidden_filenames(&path);
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if path.extension().is_some_and(|extension| extension == "rs") && name != "mod.rs" {
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            assert!(
                !RUST_KEYWORD_FILENAMES.contains(&stem.as_ref()),
                "Rust keyword used as source filename: {}",
                path.display()
            );
        }
        if FORBIDDEN_FILENAMES.contains(&name.as_ref()) {
            let relative = source_relative(&path);
            assert!(
                ALLOWED_FORBIDDEN_FILES.contains(&relative.as_str()),
                "forbidden generic file: {}",
                path.display()
            );
        }
    }
}

fn strip_rust_prefixes(mut line: &str) -> &str {
    loop {
        line = line.trim_start();
        if line.starts_with("#[") {
            let end = line.find(']').expect("Rust attribute should be closed");
            line = &line[end + 1..];
            continue;
        }
        let mut removed_visibility = false;
        for visibility in ["pub(crate)", "pub(super)", "pub(self)", "pub(in "] {
            if line.starts_with(visibility) {
                let end = line.find(')').expect("Rust visibility should be closed");
                line = &line[end + 1..];
                removed_visibility = true;
                break;
            }
        }
        if removed_visibility {
            continue;
        }
        if let Some(rest) = line.strip_prefix("pub ") {
            line = rest;
            continue;
        }
        if let Some(rest) = line.strip_prefix("const ") {
            if rest.starts_with("fn ")
                || rest.starts_with("unsafe fn ")
                || rest.starts_with("async fn ")
            {
                line = rest;
                continue;
            }
            return line;
        }
        let mut removed_modifier = false;
        for modifier in ["unsafe ", "async "] {
            if let Some(rest) = line.strip_prefix(modifier) {
                line = rest;
                removed_modifier = true;
                break;
            }
        }
        if removed_modifier {
            continue;
        }
        if line.starts_with("extern ") {
            let Some(end_quote) = line[7..].find('"') else { return line };
            let after_quote = 7 + end_quote + 1;
            line = line[after_quote..].trim_start();
            continue;
        }
        return line;
    }
}

fn rust_item_starts(source: &str) -> Vec<String> {
    source
        .lines()
        .map(strip_rust_prefixes)
        .filter(|line| {
            [
                "fn ",
                "struct ",
                "enum ",
                "union ",
                "impl ",
                "impl<",
                "trait ",
                "const ",
                "static ",
                "type ",
                "macro_rules!",
                "mod ",
            ]
            .iter()
            .any(|prefix| line.starts_with(prefix))
                && if line.starts_with("mod ") { line.contains('{') } else { true }
        })
        .map(str::to_owned)
        .collect()
}

fn assert_facade_shape(directory: &Path) {
    for path in rust_sources(directory) {
        if path.file_name().and_then(|name| name.to_str()) != Some("mod.rs") {
            continue;
        }
        let relative = source_relative(&path);
        let source = fs::read_to_string(&path).expect("module facade should be readable");
        let items = rust_item_starts(&source);
        if !items.is_empty() {
            assert!(
                ALLOWED_NON_FACADE_MOD_RS.contains(&relative.as_str()),
                "non-facade item in new module facade {relative}: {items:?}"
            );
        }
    }
}

fn assert_no_source_pairs(directory: &Path) {
    for path in rust_sources(directory) {
        if path.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let sibling_directory = path.with_file_name(stem);
        if sibling_directory.is_dir() {
            let relative = source_relative(&path);
            assert!(
                ALLOWED_RS_PAIRS.contains(&relative.as_str()),
                "new X.rs + X/ pair: {relative} and {}",
                sibling_directory.display()
            );
        }
    }
}

fn assert_no_single_file_modules(directory: &Path) {
    for path in rust_sources(directory) {
        if path.file_name().and_then(|name| name.to_str()) != Some("mod.rs") {
            continue;
        }
        let module_directory = path.parent().expect("module facade parent");
        let files = fs::read_dir(module_directory)
            .expect("module directory should be readable")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name() != "mod.rs")
            .count();
        if files == 1 {
            let relative = source_relative(module_directory);
            assert!(
                ALLOWED_SINGLE_FILE_MODULES.contains(&relative.as_str()),
                "new one-file module directory: {relative}"
            );
        }
    }
}

fn assert_no_path_module_redirection(directory: &Path) {
    for path in rust_sources(directory) {
        let source = fs::read_to_string(&path).expect("Rust source should be readable");
        if source.contains("#[path") {
            let relative = source_relative(&path);
            assert!(
                ALLOWED_PATH_MODULES.contains(&relative.as_str()),
                "#[path] module redirection is not allowed: {relative}"
            );
        }
    }
}

fn assert_no_parent_name_duplicates(directory: &Path) {
    for path in rust_sources(directory) {
        if path.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(parent) = path.parent().and_then(Path::file_name).and_then(|name| name.to_str())
        else {
            continue;
        };
        if stem == parent {
            let relative = source_relative(&path);
            assert!(
                ALLOWED_PARENT_NAME_DUPLICATES.contains(&relative.as_str()),
                "file repeats its parent module name: {relative}"
            );
        }
    }
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).expect("Rust source directory should be readable") {
        let path = entry.expect("Rust source directory entry").path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

fn assert_layer_excludes(directory: &str, forbidden: &[&str]) {
    let root = repository_path(directory);
    for path in rust_sources(&root) {
        let source = fs::read_to_string(&path).expect("Rust source should be readable");
        for token in forbidden {
            assert!(!source.contains(token), "{} contains forbidden `{token}`", path.display());
        }
    }
}

fn assert_shared_codegen_excludes_backend_crates() {
    let root = repository_path("src/codegen");
    for path in rust_sources(&root) {
        if path
            .strip_prefix(&root)
            .expect("codegen path should be below codegen root")
            .components()
            .any(|component| component.as_os_str() == "cranelift")
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("codegen source should be readable");
        for token in
            ["cranelift_codegen", "cranelift_frontend", "cranelift_module", "cranelift_object"]
        {
            assert!(!source.contains(token), "{} imports backend crate `{token}`", path.display());
        }
    }
}

#[test]
fn standard_library_directories_have_canonical_facades() {
    assert_canonical_facades(&repository_path("library/std/src"));
}

#[test]
fn generic_implementation_filenames_are_rejected() {
    assert_no_forbidden_filenames(&repository_path("src"));
    assert_no_forbidden_filenames(&repository_path("library/std/src"));
}

#[test]
fn source_layout_facades_and_boundaries_are_enforced() {
    let source = repository_path("src");
    assert_facade_shape(&source);
    assert_no_path_module_redirection(&source);
    assert_no_source_pairs(&source);
    assert_no_single_file_modules(&source);
    assert_no_parent_name_duplicates(&source);
}

#[test]
fn compiler_layers_do_not_import_reverse_pipeline_or_backend_logic() {
    assert_layer_excludes(
        "src/lexer",
        &["crate::parser", "crate::ast", "crate::semantic", "crate::codegen"],
    );
    assert_layer_excludes(
        "src/parser",
        &["crate::semantic", "crate::codegen", "OwnershipState", "BorrowRecord", "SemanticError"],
    );
    assert_layer_excludes("src/ast", &["crate::codegen", "cranelift", "NativeType"]);
    assert_layer_excludes(
        "src/semantic",
        &["crate::codegen", "cranelift", "render_diagnostic", "println!", "eprintln!"],
    );
    assert_layer_excludes(
        "src/codegen",
        &["SemanticErrorKind", "SemanticError", "Analyzer", "render_diagnostic"],
    );
}

#[test]
fn shared_codegen_excludes_backend_specific_crates() {
    assert_shared_codegen_excludes_backend_crates();
}

#[test]
fn shared_codegen_facade_uses_neutral_error_contracts() {
    let facade = fs::read_to_string(repository_path("src/codegen/mod.rs"))
        .expect("codegen facade should be readable");
    assert!(facade.contains("BackendEmitError"));
    assert!(facade.contains("BackendAbiError"));
    assert!(facade.contains("BackendSymbolBindings"));
    assert!(!facade.contains("NativeEmitError"));
    assert!(!facade.contains("NativeAbiError"));
    assert!(!facade.contains("NativeSymbolBindings"));
    assert!(!facade.contains("NativeInstruction"));
    assert!(!facade.contains("NativeCleanupPlan"));
    assert!(!facade.contains("NativeCleanupSchedule"));
}

#[test]
fn shared_codegen_facade_does_not_reexport_cranelift_emission() {
    let facade = fs::read_to_string(repository_path("src/codegen/mod.rs"))
        .expect("codegen facade should be readable");
    assert!(facade.contains("pub mod cranelift;"));
    assert!(!facade.contains("emit_program_object"));
    assert!(!facade.contains("emit_module_object"));
    assert!(!facade.contains("validate_native_signature"));
}

#[test]
fn llvm_identity_inputs_are_tracked_by_build_script() {
    let build_script =
        fs::read_to_string(repository_path("build.rs")).expect("build script should be readable");
    for source in [
        "fn selected_llvm_prefix",
        "let bundled_manifest = selected_prefix.join(\"share/actus/toolchain.json\")",
        "println!(\"cargo:rerun-if-changed={}\", bundled_manifest.display())",
        "let release_manifest = repository_root.join(\"toolchains/llvm/llvm-23.1.2.json\")",
        "println!(\"cargo:rerun-if-changed={}\", release_manifest.display())",
    ] {
        assert!(
            build_script.contains(source),
            "build script does not derive LLVM invalidation from the selected prefix: {source}"
        );
    }
    for directive in [
        "cargo:rerun-if-env-changed=CARGO_FEATURE_LLVM",
        "cargo:rerun-if-env-changed=DEP_LLVM_23_CONFIG_PATH",
        "cargo:rerun-if-env-changed=LLVM_CONFIG_PATH",
        "cargo:rerun-if-env-changed=LLVM_SYS_231_PREFIX",
    ] {
        assert!(
            build_script.contains(directive),
            "missing build invalidation directive: {directive}"
        );
    }
    assert!(!build_script.contains(
        "cargo:rerun-if-changed=.actus-toolchain/llvm-23.1.2/share/actus/toolchain.json"
    ));
}

#[test]
fn cranelift_dependencies_are_optional_and_llvm_is_default() {
    let source = fs::read_to_string(repository_path("Cargo.toml"))
        .expect("Cargo manifest should be readable");
    let manifest: toml::Value =
        toml::from_str(&source).expect("Cargo manifest should be valid TOML");
    let features = manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .expect("Cargo features should be a table");
    let default_features = features
        .get("default")
        .and_then(toml::Value::as_array)
        .expect("default Cargo features should be an array");
    assert!(default_features.iter().any(|feature| feature.as_str() == Some("llvm")));
    let cranelift_features = features
        .get("cranelift")
        .and_then(toml::Value::as_array)
        .expect("Cranelift feature should be an array");
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("Cargo dependencies should be a table");
    for dependency in
        ["cranelift-codegen", "cranelift-frontend", "cranelift-module", "cranelift-object"]
    {
        let declaration = dependencies
            .get(dependency)
            .and_then(toml::Value::as_table)
            .unwrap_or_else(|| panic!("{dependency} must use a table dependency declaration"));
        assert_eq!(
            declaration.get("optional").and_then(toml::Value::as_bool),
            Some(true),
            "{dependency} must be optional"
        );
        assert!(
            cranelift_features
                .iter()
                .any(|feature| { feature.as_str() == Some(&format!("dep:{dependency}")) }),
            "Cranelift feature must enable {dependency}"
        );
    }
}

#[test]
fn llvm_integration_is_the_default_feature() {
    let manifest = fs::read_to_string(repository_path("Cargo.toml"))
        .expect("Cargo manifest should be readable");
    assert!(manifest.contains("default = [\"llvm\"]"));
    assert!(manifest.contains("llvm = [\"dep:llvm-sys\", \"dep:object\"]"));
    assert!(manifest.contains(
        "llvm-sys = { version = \"231.0.0\", optional = true, features = [\"strict-versioning\"] }"
    ));
    assert!(!manifest.contains("inkwell = \""));
}

#[test]
fn cranelift_implementation_stays_inside_its_backend_boundary() {
    let root = repository_path("src/codegen");
    for path in rust_sources(&root) {
        let relative = path.strip_prefix(&root).expect("codegen path should be below root");
        if relative.components().any(|component| component.as_os_str() == "cranelift") {
            continue;
        }
        if relative == Path::new("mod.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("codegen source should be readable");
        for token in ["cranelift::", "cranelift_codegen", "cranelift_frontend"] {
            assert!(
                !source.contains(token),
                "{} leaks Cranelift implementation `{token}`",
                path.display()
            );
        }
    }
}

#[test]
fn target_and_runtime_layers_exclude_cranelift_implementation_types() {
    for directory in ["src/target", "src/runtime"] {
        assert_layer_excludes(
            directory,
            &["crate::codegen::cranelift", "cranelift_codegen", "cranelift_frontend"],
        );
    }
}

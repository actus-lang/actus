use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::modules::{ModuleResolver, ModuleSourceKind, analyze_module, load_module_unit};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-module-unit-{stamp}"));
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, source: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
        fs::write(path, source).expect("write fixture source");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn module_unit_preserves_identity_source_order_and_provenance() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open operations;");
    fixture.write("math/operations.act", "open verb add() -> Int { return 42; }");
    fixture.write("math/internal.act", "verb hidden() -> Int { return 7; }");

    let unit = load_module_unit(&ModuleResolver::new(&fixture.root), "math")
        .expect("module unit should load");

    assert_eq!(unit.identity().module_path(), "math");
    assert!(!unit.identity().source_key().is_empty());
    assert_eq!(unit.sources().len(), 3);
    assert!(matches!(unit.sources()[0].kind(), ModuleSourceKind::Facade));
    assert!(
        matches!(unit.sources()[1].kind(), ModuleSourceKind::Sibling { name } if name == "internal")
    );
    assert!(
        matches!(unit.sources()[2].kind(), ModuleSourceKind::Sibling { name } if name == "operations")
    );
    assert_eq!(unit.implementation().declarations.len(), 3);
    assert_eq!(
        unit.sources()[1].path().file_stem().and_then(|name| name.to_str()),
        Some("internal")
    );
}

#[test]
fn module_unit_keeps_internal_program_separate_from_public_exports() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open operations;");
    fixture.write(
        "math/operations.act",
        "open verb add() -> Int { return hidden(); } verb hidden() -> Int { return 42; }",
    );

    let unit = load_module_unit(&ModuleResolver::new(&fixture.root), "math")
        .expect("module unit should load");

    assert_eq!(unit.implementation().declarations.len(), 3);
    assert!(unit.exports().contains("verb", "add"));
    assert!(!unit.exports().contains("verb", "hidden"));
}

#[test]
fn module_unit_records_child_sources_in_hierarchical_order() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act", "open runtime;");
    fixture.write("aie/runtime/runtime.act", "open api;");
    fixture.write("aie/runtime/api.act", "open verb start() -> Int { return 42; }");

    let unit = load_module_unit(&ModuleResolver::new(&fixture.root), "aie")
        .expect("hierarchical module unit should load");

    assert_eq!(unit.sources().len(), 3);
    assert!(matches!(
        unit.sources()[1].kind(),
        ModuleSourceKind::ChildFacade { module_path } if module_path == "aie::runtime"
    ));
    assert!(matches!(
        unit.sources()[2].kind(),
        ModuleSourceKind::ChildSibling { module_path, name }
            if module_path == "aie::runtime" && name == "api"
    ));
}

#[test]
fn repeated_unit_loads_have_equal_identity_and_export_order() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open operations;");
    fixture.write("math/operations.act", "open verb add() -> Int { return 42; }");
    let resolver = ModuleResolver::new(&fixture.root);

    let first = load_module_unit(&resolver, "math").expect("first load should succeed");
    let second = load_module_unit(&resolver, "math").expect("second load should succeed");

    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.sources(), second.sources());
    assert_eq!(first.exports(), second.exports());
}

#[test]
fn internal_analysis_resolves_private_helpers_and_bridges() {
    let fixture = Fixture::new();
    fixture.write("runtime/runtime.act", "open api;");
    fixture.write(
        "runtime/api.act",
        "open unsafe extern \"C\" verb private_bridge() -> Int; open verb read() -> Int { return private_helper() + private_bridge(); } verb private_helper() -> Int { return 40; }",
    );

    analyze_module(&ModuleResolver::new(&fixture.root), "runtime")
        .expect("internal semantic scope should resolve private dependencies");
}

#[test]
fn internal_analysis_rejects_an_unresolved_private_dependency() {
    let fixture = Fixture::new();
    fixture.write("runtime/runtime.act", "open api;");
    fixture
        .write("runtime/api.act", "open verb read() -> Int { return missing_private_helper(); }");

    analyze_module(&ModuleResolver::new(&fixture.root), "runtime")
        .expect_err("unresolved internal dependencies must fail before codegen");
}

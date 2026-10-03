use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::modules::{ModuleError, ModuleResolver, exports_module};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-facade-{stamp}"));
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
fn facade_reexports_open_sibling_symbols() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act", "open registers;");
    fixture.write(
        "driver/gpio/registers.act",
        "open struct GpioBank { base_address: Int, } struct RawHardwareOffset { offset: Int, }",
    );

    let exports = exports_module(&ModuleResolver::new(&fixture.root), "driver::gpio")
        .expect("facade export should resolve");

    assert!(exports.contains("struct", "GpioBank"));
    assert!(!exports.contains("struct", "RawHardwareOffset"));
}

#[test]
fn facade_reexports_open_child_module_symbols() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act", "open runtime;");
    fixture.write("aie/runtime/runtime.act", "open api;");
    fixture.write(
        "aie/runtime/api.act",
        "open verb start() -> Int { return private_helper(); } verb private_helper() -> Int { return 7; }",
    );

    let exports = exports_module(&ModuleResolver::new(&fixture.root), "aie")
        .expect("parent facade should aggregate child exports");

    assert!(exports.contains("verb", "start"));
    assert!(!exports.contains("verb", "private_helper"));
}

#[test]
fn closed_child_module_is_not_reexported_by_parent_facade() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act", "");
    fixture.write("aie/runtime/runtime.act", "open api;");
    fixture.write("aie/runtime/api.act", "open verb start() -> Int { return 7; }");

    let exports = exports_module(&ModuleResolver::new(&fixture.root), "aie")
        .expect("parent facade should resolve without opening the child");

    assert!(!exports.contains("verb", "start"));
}

#[test]
fn rejects_duplicate_exports_from_two_child_siblings() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act", "open runtime;");
    fixture.write("aie/runtime/runtime.act", "open first; open second;");
    fixture.write("aie/runtime/first.act", "open verb start() -> Int { return 1; }");
    fixture.write("aie/runtime/second.act", "open verb start() -> Int { return 2; }");

    let error = exports_module(&ModuleResolver::new(&fixture.root), "aie")
        .expect_err("duplicate child exports must be rejected");
    let ModuleError::SymbolCollision { symbol, first_module, second_module } = error else {
        panic!("expected child export collision");
    };
    assert_eq!(symbol, "verb `start`");
    assert_eq!(first_module, "aie::runtime");
    assert_eq!(second_module, "aie::runtime");
}

#[test]
fn rejects_parent_child_export_collision() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act", "open api; open runtime;");
    fixture.write("aie/api.act", "open verb start() -> Int { return 1; }");
    fixture.write("aie/runtime/runtime.act", "open api;");
    fixture.write("aie/runtime/api.act", "open verb start() -> Int { return 2; }");

    let error = exports_module(&ModuleResolver::new(&fixture.root), "aie")
        .expect_err("parent and child export collisions must be rejected");
    let ModuleError::SymbolCollision { symbol, first_module, second_module } = error else {
        panic!("expected parent-child export collision");
    };
    assert_eq!(symbol, "verb `start`");
    assert_eq!(first_module, "aie");
    assert_eq!(second_module, "aie::runtime");
}

#[test]
fn unlisted_sibling_symbols_are_not_external_exports() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act", "open registers;");
    fixture.write("driver/gpio/registers.act", "open struct GpioBank { base: Int, }");
    fixture.write("driver/gpio/internal_helpers.act", "open struct InternalOnly { code: Int, }");

    let exports = exports_module(&ModuleResolver::new(&fixture.root), "driver::gpio")
        .expect("facade export should resolve");

    assert!(exports.contains("struct", "GpioBank"));
    assert!(!exports.contains("struct", "InternalOnly"));
}

#[test]
fn facade_controls_performance_exports_as_well() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act", "open implementations;");
    fixture.write(
        "driver/gpio/implementations.act",
        "open perform PublicRole for PublicType { } perform HiddenRole for HiddenType { }",
    );

    let exports = exports_module(&ModuleResolver::new(&fixture.root), "driver::gpio")
        .expect("performance exports should resolve");

    assert!(exports.contains("perform", "PublicRole for PublicType"));
    assert!(!exports.contains("perform", "HiddenRole for HiddenType"));
}

#[test]
fn facade_rejects_unknown_sibling_module() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act", "open missing;");

    let error = exports_module(&ModuleResolver::new(&fixture.root), "driver::gpio")
        .expect_err("unknown sibling must be rejected");

    assert!(
        matches!(&error, ModuleError::UnknownSiblingModule { sibling, .. } if sibling == "missing")
    );
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1104");
}

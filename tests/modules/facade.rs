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
fn facade_rejects_unknown_sibling_module() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act", "open missing;");

    let error = exports_module(&ModuleResolver::new(&fixture.root), "driver::gpio")
        .expect_err("unknown sibling must be rejected");

    assert!(
        matches!(error, ModuleError::UnknownSiblingModule { sibling, .. } if sibling == "missing")
    );
}

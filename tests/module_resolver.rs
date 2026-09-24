use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::module_resolver::{ModuleResolutionError, ModuleResolver};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-modules-{stamp}"));
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn write(&self, relative: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
        fs::write(path, "").expect("write fixture source");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn names(paths: &[PathBuf]) -> Vec<&str> {
    paths.iter().map(|path| path.file_name().unwrap().to_str().unwrap()).collect()
}

#[test]
fn resolves_directory_facade_and_sorted_siblings() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act");
    fixture.write("driver/gpio/z_ops.act");
    fixture.write("driver/gpio/a_registers.act");
    fixture.write("driver/gpio/ignored.txt");
    fixture.write("driver/gpio/nested/hidden.act");

    let module = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect("directory module should resolve");

    assert_eq!(module.module_path(), "driver::gpio");
    assert_eq!(module.facade(), fixture.root.join("driver/gpio/gpio.act").as_path());
    assert_eq!(names(module.siblings()), vec!["a_registers.act", "z_ops.act"]);
    assert_eq!(module.source_files().count(), 3);
}

#[test]
fn resolves_single_file_module_without_siblings() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio.act");

    let module = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect("single-file module should resolve");

    assert_eq!(module.facade(), fixture.root.join("driver/gpio.act").as_path());
    assert!(module.siblings().is_empty());
}

#[test]
fn rejects_directory_without_canonical_facade() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/registers.act");

    let error = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect_err("missing facade should fail");

    assert!(matches!(error, ModuleResolutionError::MissingFacade { .. }));
}

#[test]
fn rejects_ambiguous_directory_and_single_file_roots() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act");
    fixture.write("driver/gpio.act");

    let error = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect_err("ambiguous module roots should fail");

    assert!(matches!(error, ModuleResolutionError::AmbiguousModule { .. }));
}

#[test]
fn rejects_invalid_module_paths() {
    let fixture = Fixture::new();
    let error = ModuleResolver::new(&fixture.root)
        .resolve("driver/../gpio")
        .expect_err("path traversal must fail");
    assert!(matches!(error, ModuleResolutionError::InvalidPath(_)));
}

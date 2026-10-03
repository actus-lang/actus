use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::modules::{ModuleResolutionError, ModuleResolver};

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
    fixture.write("driver/gpio/nested/nested.act");

    let module = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect("directory module should resolve");

    assert_eq!(module.module_path(), "driver::gpio");
    assert_eq!(module.facade(), fixture.root.join("driver/gpio/gpio.act").as_path());
    assert_eq!(names(module.siblings()), vec!["a_registers.act", "z_ops.act"]);
    assert_eq!(module.source_files().count(), 3);
    assert_eq!(module.children().len(), 1);
    assert_eq!(module.children()[0].module_path(), "driver::gpio::nested");
    assert_eq!(module.children()[0].facade(), fixture.root.join("driver/gpio/nested/nested.act"));
}

#[test]
fn discovers_sorted_hierarchical_child_directories() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act");
    fixture.write("aie/layout/layout.act");
    fixture.write("aie/runtime/runtime.act");
    fixture.write("aie/runtime/fabric.act");
    fixture.write("aie/persistence/persistence.act");

    let module =
        ModuleResolver::new(&fixture.root).resolve("aie").expect("parent facade should resolve");

    assert_eq!(module.facade(), fixture.root.join("aie/aie.act").as_path());
    assert!(module.siblings().is_empty());
    assert_eq!(module.source_files().count(), 1);
    assert_eq!(module.children().len(), 3);
    assert_eq!(
        module.children().iter().map(|child| child.module_path()).collect::<Vec<_>>(),
        vec!["aie::layout", "aie::persistence", "aie::runtime"]
    );
}

#[test]
fn rejects_child_directory_without_a_canonical_facade() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act");
    fixture.write("aie/runtime/fabric.act");

    let error = ModuleResolver::new(&fixture.root)
        .resolve("aie")
        .expect_err("child directory without a facade should fail");

    assert!(matches!(
        error,
        ModuleResolutionError::MissingFacade { module, .. } if module == "aie::runtime"
    ));
}

#[test]
fn rejects_child_directory_that_conflicts_with_a_direct_sibling() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act");
    fixture.write("aie/runtime.act");
    fixture.write("aie/runtime/runtime.act");

    let error = ModuleResolver::new(&fixture.root)
        .resolve("aie")
        .expect_err("child directory and sibling file should conflict");

    assert!(matches!(
        error,
        ModuleResolutionError::AmbiguousModule { module, .. } if module == "aie::runtime"
    ));
}

#[test]
fn records_hierarchical_module_baseline_and_rejects_child_facade_bypass() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act");
    fixture.write("aie/runtime/runtime.act");

    let error = ModuleResolver::new(&fixture.root)
        .resolve("aie::runtime")
        .expect_err("child modules must be reached through the parent facade");

    assert!(
        matches!(&error, ModuleResolutionError::BypassesFacade { parent, .. } if parent == "aie")
    );
    let diagnostic =
        actus::diagnostics::module_diagnostic(&actus::modules::ModuleError::Resolution(error));
    assert_eq!(diagnostic.code(), "E1108");
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
fn rejects_direct_imports_that_bypass_a_parent_facade() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act");
    fixture.write("driver/gpio/registers.act");

    let error = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio::registers")
        .expect_err("sibling modules must be reached through the parent facade");

    assert!(
        matches!(&error, ModuleResolutionError::BypassesFacade { parent, .. } if parent == "driver::gpio")
    );
    let diagnostic =
        actus::diagnostics::module_diagnostic(&actus::modules::ModuleError::Resolution(error));
    assert_eq!(diagnostic.code(), "E1108");
}

#[test]
fn rejects_invalid_module_paths() {
    let fixture = Fixture::new();
    let error = ModuleResolver::new(&fixture.root)
        .resolve("driver/../gpio")
        .expect_err("path traversal must fail");
    assert!(matches!(error, ModuleResolutionError::InvalidPath(_)));
}

#[test]
fn classifies_resolution_failures_with_stable_module_codes() {
    let fixture = Fixture::new();
    let missing = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect_err("missing facade should fail");
    let diagnostic =
        actus::diagnostics::module_diagnostic(&actus::modules::ModuleError::Resolution(missing));
    assert_eq!(diagnostic.code(), "E1101");
}

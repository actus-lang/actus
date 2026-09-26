use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use actus::configuration::CompilerConfiguration;
use actus::lexer::scan;
use actus::modules::{ModuleResolver, resolve_imports};
use actus::parser::parse;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-actus-manifest-{stamp}"));
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, source: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create parent");
        fs::write(path, source).expect("write fixture");
    }

    fn manifest(&self, source_root: Option<&str>) {
        let source_root = source_root.map(|root| format!("source_root = \"{root}\"\n"));
        self.write(
            "Actus.toml",
            &format!(
                "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n{}",
                source_root.unwrap_or_default()
            ),
        );
    }

    fn legacy_manifest(&self) {
        self.write("Arca.toml", "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn imported_program(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("importing source should parse")
}

fn assert_import_resolves(fixture: &Fixture, source_root: &Path) {
    let configuration = CompilerConfiguration::from_input_path(&fixture.root.join("main.act"))
        .expect("manifest should be discovered from input path");
    assert_eq!(configuration.source_root(), source_root);
    let program = imported_program("import calc;");
    resolve_imports(&program, &ModuleResolver::new(configuration.source_root()))
        .expect("module import should resolve from manifest source root");
}

#[test]
fn resolves_imports_from_default_src_root() {
    let fixture = Fixture::new();
    fixture.manifest(None);
    fixture.write("main.act", "");
    fixture.write("src/calc/calc.act", "open ops;");
    fixture.write(
        "src/calc/ops.act",
        "open verb add(erg left: Int, erg right: Int) -> Int { return left + right; }",
    );

    assert_import_resolves(&fixture, &fixture.root.join("src"));
}

#[test]
fn resolves_imports_from_custom_source_root() {
    let fixture = Fixture::new();
    fixture.manifest(Some("modules"));
    fixture.write("main.act", "");
    fixture.write("modules/calc/calc.act", "open ops;");
    fixture.write(
        "modules/calc/ops.act",
        "open verb add(erg left: Int, erg right: Int) -> Int { return left + right; }",
    );

    assert_import_resolves(&fixture, &fixture.root.join("modules"));
}

#[test]
fn rejects_missing_custom_source_root() {
    let fixture = Fixture::new();
    fixture.manifest(Some("missing"));
    let error = CompilerConfiguration::from_input_path(&fixture.root.join("main.act"))
        .expect_err("missing source root must fail");
    assert!(error.to_string().contains("InvalidSourceRoot"));
}

#[test]
fn resolves_legacy_manifest_with_compatibility_fallback() {
    let fixture = Fixture::new();
    fixture.legacy_manifest();
    fixture.write("main.act", "");
    fixture.write("src/calc/calc.act", "open ops;");
    fixture.write(
        "src/calc/ops.act",
        "open verb add(erg left: Int, erg right: Int) -> Int { return left + right; }",
    );

    assert_import_resolves(&fixture, &fixture.root.join("src"));
}

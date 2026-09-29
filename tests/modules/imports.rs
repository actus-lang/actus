use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::lexer::scan;
use actus::modules::{ModuleError, ModuleResolver, analyze_with_imports};
use actus::parser::parse;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-import-{stamp}"));
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

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

#[test]
fn resolves_only_facade_exports_into_the_importing_unit() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act", "open registers;");
    fixture.write(
        "driver/gpio/registers.act",
        "open struct GpioBank { base: Int, } struct HiddenBank { base: Int, }",
    );
    let program = parse_source(
        "import driver::gpio; verb main() -> Int { erg bank = GpioBank { base: 7, }; return bank.base; }",
    );

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("facade exports should be visible to the importing unit");
}

#[test]
fn rejects_private_imported_types() {
    let fixture = Fixture::new();
    fixture.write("driver/gpio/gpio.act", "open registers;");
    fixture.write("driver/gpio/registers.act", "struct HiddenBank { base: Int, }");
    let program = parse_source(
        "import driver::gpio; verb main() -> Int { erg bank = HiddenBank { base: 7, }; return bank.base; }",
    );

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("closed sibling declarations must not cross the import boundary");
    assert!(
        matches!(error, ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "HiddenBank")
    );
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1109");
}

#[test]
fn rejects_direct_calls_to_private_runtime_bridges() {
    let fixture = Fixture::new();
    fixture.write("runtime/runtime.act", "open api;");
    fixture.write(
        "runtime/api.act",
        "open verb read() -> Int { return private_bridge(); } unsafe extern \"C\" verb private_bridge() -> Int;",
    );
    let program = parse_source("import runtime; verb main() -> Int { return private_bridge(); }");

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("raw runtime bridge must remain private");
    assert!(
        matches!(error, ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "private_bridge")
    );
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1109");
}

#[test]
fn resolves_repeated_imports_once() {
    let fixture = Fixture::new();
    fixture.write("shared/shared.act", "open types;");
    fixture.write("shared/types.act", "open struct Token { byte: Int, }");
    let program = parse_source(
        "import shared; import shared; verb main() -> Int { erg token = Token { byte: 7, }; return token.byte; }",
    );

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("repeated imports must not duplicate exported declarations");
}

#[test]
fn rejects_unknown_import_with_stable_module_diagnostic() {
    let fixture = Fixture::new();
    let program = parse_source("import missing; verb main() -> Int { return 0; }");

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("unknown imports must fail module resolution");
    let diagnostic = actus::diagnostics::module_diagnostic(&error);
    assert_eq!(diagnostic.code(), "E1101");
}

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
    assert!(matches!(error, ModuleError::Semantic(_)));
}

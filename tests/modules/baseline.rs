use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::lexer::scan;
use actus::modules::{
    ModuleResolver, analyze_module, analyze_with_imports, exports_module, parse_module,
};
use actus::parser::parse;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-module-baseline-{stamp}"));
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
fn baseline_records_flattened_internal_program_and_public_exports() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open operations;");
    fixture.write(
        "math/operations.act",
        "open verb add() -> Int { return hidden_constant(); } verb hidden_constant() -> Int { return 42; }",
    );
    let resolver = ModuleResolver::new(&fixture.root);
    let program = parse_module(&resolver, "math").expect("module should parse");
    let exports = exports_module(&resolver, "math").expect("exports should resolve");

    assert_eq!(program.declarations.len(), 3);
    assert!(exports.contains("verb", "add"));
    assert!(!exports.contains("verb", "hidden_constant"));
}

#[test]
fn baseline_internal_scope_resolves_private_helper_from_public_wrapper() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open operations;");
    fixture.write(
        "math/operations.act",
        "open verb add() -> Int { return hidden_constant(); } verb hidden_constant() -> Int { return 42; }",
    );

    analyze_module(&ModuleResolver::new(&fixture.root), "math")
        .expect("current internal aggregation must resolve private helper");
}

#[test]
fn baseline_external_scope_rejects_private_helper_call() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open operations;");
    fixture.write(
        "math/operations.act",
        "open verb add() -> Int { return hidden_constant(); } verb hidden_constant() -> Int { return 42; }",
    );
    let program = parse_source("import math; verb main() -> Int { return hidden_constant(); }");

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("private helper must not enter the importer namespace");
}

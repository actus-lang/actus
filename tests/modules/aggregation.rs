use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::modules::{ModuleError, ModuleResolver, analyze_module, parse_module};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-module-ast-{stamp}"));
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
fn combines_sibling_asts_into_one_semantic_scope() {
    let fixture = Fixture::new();
    fixture.write(
        "demo/demo.act",
        "verb main() -> Int { erg item = Shared { number: 7, }; return item.number; }",
    );
    fixture.write("demo/types.act", "struct Shared { number: Int, }");

    let resolver = ModuleResolver::new(&fixture.root);
    let program = parse_module(&resolver, "demo").expect("sibling sources should parse together");
    assert_eq!(program.declarations.len(), 2);
    analyze_module(&resolver, "demo").expect("sibling type should be visible without import");
}

#[test]
fn reports_duplicate_declarations_with_both_source_locations() {
    let fixture = Fixture::new();
    fixture.write("demo/demo.act", "struct Shared { number: Int, }");
    fixture.write("demo/other.act", "struct Shared { number: Int, }");

    let error = parse_module(&ModuleResolver::new(&fixture.root), "demo")
        .expect_err("duplicate sibling declarations must fail");
    let ModuleError::DuplicateDeclaration(diagnostic) = error else {
        panic!("expected duplicate declaration diagnostic");
    };
    assert_eq!(diagnostic.kind, "struct");
    assert_eq!(diagnostic.name, "Shared");
    assert_eq!(diagnostic.first.path, fixture.root.join("demo/demo.act"));
    assert_eq!(diagnostic.second.path, fixture.root.join("demo/other.act"));
    assert_eq!(diagnostic.first.span.start, 0);
    assert_eq!(diagnostic.second.span.start, 0);
}

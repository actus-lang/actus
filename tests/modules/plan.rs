use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::lexer::scan;
use actus::modules::{ModuleError, ModuleObjectOwner, ModuleResolver, build_compilation_plan};
use actus::parser::parse;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-plan-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn write(&self, relative: &str, source: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn compilation_plan_separates_public_caller_and_internal_module_units() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open ops;");
    fixture.write(
        "math/ops.act",
        "verb hidden() -> Int { return 41; } open verb add() -> Int { return hidden() + 1; }",
    );
    let (tokens, errors) = scan("import math; verb main() -> Int { return add(); }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0)).unwrap();
    assert_eq!(plan.units().len(), 1);
    assert!(plan.caller().declarations.iter().all(|declaration| {
        !matches!(declaration, actus::ast::TopLevelDecl::Verb(verb) if verb.name == "hidden")
    }));
    assert!(plan.units()[0].implementation().declarations.iter().any(|declaration| {
        matches!(declaration, actus::ast::TopLevelDecl::Verb(verb) if verb.name == "hidden")
    }));
}

#[test]
fn compilation_plan_deduplicates_repeated_module_imports() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open ops;");
    fixture.write("math/ops.act", "open verb add() -> Int { return 1; }");
    let (tokens, errors) = scan("import math; import math; verb main() -> Int { return add(); }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0)).unwrap();
    assert_eq!(plan.units().len(), 1);
    assert_eq!(
        plan.caller()
            .declarations
            .iter()
            .filter(|declaration| {
                matches!(
                    declaration,
                    actus::ast::TopLevelDecl::Verb(verb) if verb.name == "add"
                ) || matches!(
                    declaration,
                    actus::ast::TopLevelDecl::ExternalVerb(verb) if verb.name == "add"
                )
            })
            .count(),
        1
    );
}

#[test]
fn rejects_ambiguous_native_symbols_before_codegen() {
    let fixture = Fixture::new();
    fixture.write("left/left.act", "open api;");
    fixture.write("left/api.act", "open verb shared() -> Int { return 1; }");
    fixture.write("right/right.act", "open api;");
    fixture.write("right/api.act", "open verb shared() -> Int { return 2; }");
    let (tokens, errors) = scan("import left; import right; verb main() -> Int { return 0; }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let error = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0))
        .expect_err("ambiguous implementation symbols must fail before codegen");
    assert!(
        matches!(error, ModuleError::SymbolCollision { ref symbol, .. } if symbol == "verb `shared`")
    );
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1110");
}

#[test]
fn deduplicates_compatible_external_symbols_across_modules() {
    let fixture = Fixture::new();
    fixture.write("left/left.act", "open bridge;");
    fixture.write(
        "left/bridge.act",
        "unsafe extern \"C\" verb shared_bridge(abs buffer: Buffer) -> Int;",
    );
    fixture.write("right/right.act", "open bridge;");
    fixture.write(
        "right/bridge.act",
        "unsafe extern \"C\" verb shared_bridge(abs buffer: Buffer) -> Int;",
    );
    let (tokens, errors) = scan("import left; import right; verb main() -> Int { return 0; }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    build_compilation_plan(&program, &ModuleResolver::new(&fixture.0))
        .expect("compatible external ABI declarations should be shared");
}

#[test]
fn rejects_incompatible_external_symbols_across_modules() {
    let fixture = Fixture::new();
    fixture.write("left/left.act", "open bridge;");
    fixture.write(
        "left/bridge.act",
        "unsafe extern \"C\" verb shared_bridge(abs buffer: Buffer) -> Int;",
    );
    fixture.write("right/right.act", "open bridge;");
    fixture.write(
        "right/bridge.act",
        "unsafe extern \"C\" verb shared_bridge(ins buffer: Buffer) -> Int;",
    );
    let (tokens, errors) = scan("import left; import right; verb main() -> Int { return 0; }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let error = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0))
        .expect_err("incompatible external ABI declarations must fail");
    assert!(
        matches!(error, ModuleError::SymbolCollision { ref symbol, .. } if symbol == "native symbol `shared_bridge`")
    );
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1110");
}

#[test]
fn object_plan_orders_root_and_imported_units_by_canonical_module_path() {
    let fixture = Fixture::new();
    fixture.write("zeta/zeta.act", "open api;");
    fixture.write("zeta/api.act", "open verb zed() -> Int { return 1; }");
    fixture.write("alpha/alpha.act", "open api;");
    fixture.write("alpha/api.act", "open verb aye() -> Int { return 2; }");
    let (tokens, errors) =
        scan("import zeta; import alpha; verb main() -> Int { return zed() + aye(); }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0)).unwrap();
    let objects = plan.object_plan().unwrap();
    assert!(matches!(objects.units()[0].owner(), ModuleObjectOwner::Root));
    assert!(matches!(
        objects.units()[1].owner(),
        ModuleObjectOwner::Imported { module_path } if module_path == "alpha"
    ));
    assert!(matches!(
        objects.units()[2].owner(),
        ModuleObjectOwner::Imported { module_path } if module_path == "zeta"
    ));
}

#[test]
fn object_metadata_lists_only_facade_exported_verbs() {
    let fixture = Fixture::new();
    fixture.write("runtime/runtime.act", "open api;");
    fixture.write(
        "runtime/api.act",
        "open verb read() -> Int { return private_bridge(); } verb private_bridge() -> Int { return 1; }",
    );
    let (tokens, errors) = scan("import runtime; verb main() -> Int { return read(); }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0)).unwrap();
    let objects = plan.object_plan().unwrap();
    let exported = objects.units()[1].exported_verbs();
    assert_eq!(exported, &["read".to_owned()]);
}

#[test]
fn imported_object_program_excludes_module_directives() {
    let fixture = Fixture::new();
    fixture.write("math/math.act", "open ops;");
    fixture.write("math/ops.act", "open verb add() -> Int { return 1; }");
    let (tokens, errors) = scan("import math; verb main() -> Int { return add(); }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0)).unwrap();
    let objects = plan.object_plan().unwrap();
    let object = objects.units()[1].program();
    assert!(object.declarations.iter().all(|declaration| {
        !matches!(declaration, actus::ast::TopLevelDecl::Import(_))
            && !matches!(declaration, actus::ast::TopLevelDecl::OpenSibling(_))
    }));
}

#[test]
fn hierarchical_child_implementation_has_one_parent_object_owner() {
    let fixture = Fixture::new();
    fixture.write("device/device.act", "open runtime;");
    fixture.write("device/runtime/runtime.act", "open api;");
    fixture.write(
        "device/runtime/api.act",
        "verb private_helper() -> Int { return 41; } open verb read() -> Int { return private_helper() + 1; }",
    );
    let (tokens, errors) = scan("import device; verb main() -> Int { return read(); }");
    assert!(errors.is_empty());
    let program = parse(tokens).unwrap();

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.0)).unwrap();
    let objects = plan.object_plan().unwrap();
    assert_eq!(objects.units().len(), 2);
    assert!(matches!(
        objects.units()[1].owner(),
        ModuleObjectOwner::Imported { module_path } if module_path == "device"
    ));
    assert!(objects.units()[1].program().declarations.iter().any(|declaration| {
        matches!(declaration, actus::ast::TopLevelDecl::Verb(verb) if verb.name == "private_helper")
    }));
    assert_eq!(objects.units()[1].exported_verbs(), &["read".to_owned()]);
}

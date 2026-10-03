use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::lexer::scan;
use actus::modules::{
    ModuleError, ModuleResolver, analyze_with_imports, load_module_unit, parse_module,
};
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
fn resolves_public_child_declarations_through_the_parent_facade() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act", "open runtime;");
    fixture.write("aie/runtime/runtime.act", "open api;");
    fixture.write("aie/runtime/api.act", "open verb start() -> Int { return 7; }");
    let program = parse_source("import aie; verb main() -> Int { return start(); }");

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("child exports should be visible through the parent facade");
}

#[test]
fn resolves_a_configuration_facade_with_compile_time_siblings() {
    let fixture = Fixture::new();
    fixture.write("config/config.act", "open values;");
    fixture.write("config/values.act", "open const LIMIT: u8 = 30u8;");

    parse_module(&ModuleResolver::new(&fixture.root), "config")
        .expect("configuration facade should resolve its sibling constants");
}

#[test]
fn rejects_imports_inside_the_configuration_subtree() {
    let fixture = Fixture::new();
    fixture.write("config/config.act", "import aie; open values;");
    fixture.write("config/values.act", "open const LIMIT: u8 = 30u8;");
    fixture.write("aie/aie.act", "");

    let error = parse_module(&ModuleResolver::new(&fixture.root), "config")
        .expect_err("configuration sources must not import modules");
    assert!(
        matches!(error, ModuleError::ConfigurationImport { ref module, .. } if module == "config")
    );
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1113");
}

#[test]
fn rejects_imports_inside_nested_configuration_sources() {
    let fixture = Fixture::new();
    fixture.write("config/config.act", "open policy;");
    fixture.write("config/policy/policy.act", "import aie;");
    fixture.write("aie/aie.act", "");

    let error = parse_module(&ModuleResolver::new(&fixture.root), "config")
        .expect_err("nested configuration sources must not import modules");
    assert!(
        matches!(error, ModuleError::ConfigurationImport { ref module, .. } if module == "config::policy")
    );
}

#[test]
fn rejects_direct_configuration_child_bypass() {
    let fixture = Fixture::new();
    fixture.write("config/config.act", "open values;");
    fixture.write("config/values/values.act", "open const LIMIT: u8 = 30u8;");

    let error = ModuleResolver::new(&fixture.root)
        .resolve("config::values")
        .expect_err("configuration children must be reached through the facade");
    assert!(
        matches!(error, actus::modules::ModuleResolutionError::BypassesFacade { ref parent, .. } if parent == "config")
    );
}

#[test]
fn resolves_generic_nested_calls_through_parent_facade() {
    let fixture = Fixture::new();
    fixture.write("device/device.act", "open runtime;");
    fixture.write("device/runtime/runtime.act", "open api;");
    fixture.write(
        "device/runtime/api.act",
        "open struct Packet[T] { item: T, } open verb inner[T](erg value: T) -> Packet[T] { return Packet[T] { item: value, }; } open verb outer[T](erg value: T) -> Packet[T] { return inner(value: erg value); }",
    );
    let program = parse_source(
        "import device; verb main() -> Int { erg value = 41; erg packet = outer(value: erg value); return packet.item + 1; }",
    );

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("generic nested calls should resolve through the parent facade");
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
fn rejects_private_pack_literals_through_parent_facade() {
    let fixture = Fixture::new();
    fixture.write("device/device.act", "open runtime;");
    fixture.write("device/runtime/runtime.act", "open api;");
    fixture.write(
        "device/runtime/api.act",
        "pack HiddenRegister { erg storage: u8; layout little; fields { erg ready: u1 at 0; } }",
    );
    let program = parse_source(
        "import device; verb main() -> Int { erg register = HiddenRegister { storage: 0u8, }; return 0; }",
    );

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("private pack declarations must not cross the parent facade");
    assert!(matches!(
        error,
        ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "HiddenRegister"
    ));
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1109");
}

#[test]
fn rejects_private_constants_through_parent_facade() {
    let fixture = Fixture::new();
    fixture.write("device/device.act", "open runtime;");
    fixture.write("device/runtime/runtime.act", "open api;");
    fixture.write("device/runtime/api.act", "const SECRET: u8 = 7u8;");
    let program = parse_source("import device; verb main() -> Int { return SECRET as Int; }");

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("private constants must not cross the parent facade");
    assert!(matches!(
        error,
        ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "SECRET"
    ));
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
fn rejects_private_types_in_exported_signatures() {
    let fixture = Fixture::new();
    fixture.write("api/api.act", "open surface;");
    fixture.write(
        "api/surface.act",
        "struct Hidden { state: Int, } open verb expose(abs value: Hidden) -> Int { return 0; }",
    );

    let error = load_module_unit(&ModuleResolver::new(&fixture.root), "api")
        .expect_err("private parameter types must not leak through an exported verb");
    assert!(matches!(
        error,
        ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "Hidden"
    ));
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1109");
}

#[test]
fn rejects_private_roles_in_exported_generic_bounds() {
    let fixture = Fixture::new();
    fixture.write("api/api.act", "open surface;");
    fixture.write(
        "api/surface.act",
        "role HiddenRole { verb inspect(abs self: Int); } open verb expose[T: HiddenRole](erg value: T) -> Int { return 0; }",
    );

    let error = load_module_unit(&ModuleResolver::new(&fixture.root), "api")
        .expect_err("private generic bounds must not leak through an exported verb");
    assert!(matches!(
        error,
        ModuleError::PrivateDeclarationAccess { ref symbol, .. } if symbol == "HiddenRole"
    ));
}

#[test]
fn accepts_public_types_nested_in_exported_result_signatures() {
    let fixture = Fixture::new();
    fixture.write("api/api.act", "open surface;");
    fixture.write(
        "api/surface.act",
        "open enum Status { Ready, } open verb expose() -> Result[Status, Status] { return Result[Status, Status].Ok(Status.Ready); }",
    );

    load_module_unit(&ModuleResolver::new(&fixture.root), "api")
        .expect("public nested result types should remain visible");
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

#[test]
fn rejects_malformed_facade_source_before_external_type_resolution() {
    let fixture = Fixture::new();
    fixture.write("api/api.act", "open surface;");
    fixture.write("api/surface.act", "open struct Broken {");

    let error = load_module_unit(&ModuleResolver::new(&fixture.root), "api")
        .expect_err("malformed facade exports must fail before import analysis");
    assert!(matches!(error, ModuleError::Parse { .. }));
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E0004");
}

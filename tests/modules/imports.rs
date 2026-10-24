use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::codegen::cranelift::emit_program_object;
use actus::lexer::scan;
use actus::modules::{
    ModuleError, ModuleResolver, analyze_with_imports, build_compilation_plan, load_module_unit,
    parse_module,
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
fn records_imported_namespace_and_resolves_qualified_call() {
    let fixture = Fixture::new();
    fixture.write("shared/shared.act", "open api;");
    fixture.write("shared/api.act", "open verb read() -> Int { return 7; }");
    let program = parse_source("import shared; verb main() -> Int { return shared::read(); }");

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("qualified call should resolve through the imported namespace");
}

#[test]
fn rejects_unqualified_call_through_a_closed_import() {
    let fixture = Fixture::new();
    fixture.write("shared/shared.act", "open api;");
    fixture.write("shared/api.act", "open verb read() -> Int { return 7; }");
    let program = parse_source("import shared; verb main() -> Int { return read(); }");

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("closed imports must not inject unqualified declarations");
    assert!(matches!(
        error,
        ModuleError::Semantic(ref semantic)
            if matches!(semantic.kind, actus::semantic::SemanticErrorKind::UnknownVerb { ref name } if name == "read")
    ));
}

#[test]
fn resolves_qualified_call_through_a_namespace_alias() {
    let fixture = Fixture::new();
    fixture.write("shared/shared.act", "open api;");
    fixture.write("shared/api.act", "open verb read() -> Int { return 7; }");
    let program =
        parse_source("import shared as telem; verb main() -> Int { return telem::read(); }");

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("qualified alias should resolve through the imported namespace");
}

#[test]
fn rejects_qualified_call_through_an_unimported_namespace() {
    let fixture = Fixture::new();
    fixture.write("shared/shared.act", "open api;");
    fixture.write("shared/api.act", "open verb read() -> Int { return 7; }");
    let program = parse_source("import shared; verb main() -> Int { return missing::read(); }");

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("unknown namespace must be rejected");
    assert!(matches!(
        error,
        ModuleError::Semantic(ref semantic)
            if matches!(semantic.kind, actus::semantic::SemanticErrorKind::UnknownVerb { ref name } if name == "missing::read")
    ));
}

#[test]
fn resolves_public_child_declarations_through_the_parent_facade() {
    let fixture = Fixture::new();
    fixture.write("aie/aie.act", "open runtime;");
    fixture.write("aie/runtime/runtime.act", "open api;");
    fixture.write("aie/runtime/api.act", "open verb start() -> Int { return 7; }");
    let program = parse_source("import { open aie }; verb main() -> Int { return start(); }");

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("child exports should be visible through the parent facade");
}

#[test]
fn accepts_equality_for_a_unit_enum_reexported_through_a_facade() {
    let fixture = Fixture::new();
    fixture.write("teaching/teaching.act", "open contract;");
    fixture.write(
        "teaching/contract.act",
        "open enum TeachingMode { Direct, Replay, } open verb accepts(erg mode: TeachingMode) -> Bool { return mode == TeachingMode.Direct; }",
    );
    let program = parse_source(
        "import { open teaching }; verb main() -> Bool { erg mode = TeachingMode.Direct; return accepts(mode: erg mode); }",
    );

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("re-exported unit enum equality should be accepted");
}

#[test]
fn lowers_generic_option_return_through_a_facade() {
    let fixture = Fixture::new();
    fixture.write("dataset/dataset.act", "open manifest;");
    fixture.write("dataset/validation.act", "open enum DatasetError { Invalid, }");
    fixture.write(
        "dataset/manifest.act",
        "open validation; open verb read() -> Option[DatasetError] { return Option[DatasetError].Some(DatasetError.Invalid); }",
    );
    let program = parse_source(
        "import dataset; verb main() -> Int { erg header = read(); return case dat header { Option.Some(_) => 0, Option.None => 1, }; }",
    );

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.root))
        .expect("facade plan should build");
    let object = emit_program_object(&plan.implementation(), "main")
        .expect("generic Option return through a facade should emit");
    object::File::parse(object.as_slice()).expect("facade Option object should parse");
}

#[test]
fn lowers_nested_generic_option_payload_through_a_facade() {
    let fixture = Fixture::new();
    fixture.write("dataset/dataset.act", "open manifest;");
    fixture.write("dataset/validation.act", "open enum DatasetError { Invalid, }");
    fixture.write(
        "dataset/manifest.act",
        "open validation; open verb read_nested() -> Result[Option[DatasetError], DatasetError] { return Result[Option[DatasetError], DatasetError].Ok(Option[DatasetError].Some(DatasetError.Invalid)); }",
    );
    let program = parse_source(
        "import dataset; verb main() -> Int { erg result = dataset::read_nested(); return case dat result { Result.Ok(option) => case dat option { Option.Some(_) => 0, Option.None => 1, }, Result.Err(_) => 2, }; }",
    );

    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.root))
        .expect("nested facade plan should build");
    let object = emit_program_object(&plan.implementation(), "main")
        .expect("nested generic Option payload should emit");
    object::File::parse(object.as_slice()).expect("nested facade Option object should parse");
}

#[test]
fn keeps_inferred_types_scoped_across_imported_declarations() {
    let fixture = Fixture::new();
    fixture.write("layout/layout.act", "open fixed;");
    fixture.write(
        "layout/fixed.act",
        "open struct FixedHeaderLayout { value: Int, } open verb read_fixed_header_layout() -> FixedHeaderLayout { return FixedHeaderLayout { value: 1, }; }",
    );
    fixture.write("aux/aux.act", "open forwarding;");
    fixture.write(
        "aux/forwarding.act",
        "open enum AuxLinkError { Capacity, } open verb allocate_aux_record(erg value: Int) -> Option[Int] { return Option[Int].Some(value); } open verb acquire_aux_record(erg value: Int) -> Result[Int, AuxLinkError] { erg allocated = allocate_aux_record(value: erg value); erg selected: Int = case dat allocated { Option.None => 0, Option.Some(item) => item, }; erg layout = read_fixed_header_layout(); return Result[Int, AuxLinkError].Ok(selected + layout.value); }",
    );
    let program = parse_source(
        "import { open layout, open aux }; verb main() -> Int { erg input: Int = 1; erg result = acquire_aux_record(value: erg input); return case dat result { Result.Ok(value) => value, Result.Err(_) => 0, }; }",
    );

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("inferred types from imported declarations should not collide");
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
fn rejects_runtime_declarations_inside_configuration_sources() {
    let fixture = Fixture::new();
    fixture.write("config/config.act", "open values;");
    fixture.write("config/values.act", "open verb runtime_state() -> Int { return 1; }");

    let error = load_module_unit(&ModuleResolver::new(&fixture.root), "config")
        .expect_err("runtime config declaration must fail");
    assert!(
        matches!(error, ModuleError::ConfigurationRuntimeDeclaration { ref kind, .. } if kind == "verb")
    );
    let diagnostic = actus::diagnostics::module_diagnostic(&error);
    assert_eq!(diagnostic.code(), "E1114");
}

#[test]
fn accepts_transitive_public_configuration_constants_semantically() {
    let fixture = Fixture::new();
    fixture.write("config/config.act", "open values;");
    fixture.write("config/values.act", "open const LIMIT: u8 = 30u8;");
    fixture.write("main.act", "import config; verb main() -> Int { return LIMIT as Int; }");

    let program = parse_source("import config; verb main() -> Int { return LIMIT as Int; }");
    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("transitive public configuration constants should be semantic inputs");
}

#[test]
fn resolves_qualified_configuration_constant_semantically() {
    let fixture = Fixture::new();
    fixture.write("config/config.act", "open values;");
    fixture.write("config/values.act", "open const LIMIT: u8 = 30u8;");
    let program =
        parse_source("import config; verb main() -> Int { return config::LIMIT as Int; }");

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("qualified configuration constants should resolve semantically");
    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.root))
        .expect("qualified configuration constant plan should build");
    emit_program_object(&plan.implementation(), "main")
        .expect("qualified configuration constant should lower");
}

#[test]
fn resolves_qualified_enum_type_and_variant_semantically() {
    let fixture = Fixture::new();
    fixture.write("colors/colors.act", "open values;");
    fixture.write("colors/values.act", "open enum Color { Red, Blue, }");
    let program = parse_source(
        "import colors; verb main() -> Int { erg color: colors::Color = colors::Color.Red; return 0; }",
    );

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("qualified enum types and variants should resolve semantically");
    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.root))
        .expect("qualified enum plan should build");
    emit_program_object(&plan.implementation(), "main")
        .expect("qualified enum type and variant should lower");
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
        "import device; verb main() -> Int { erg value = 41; erg packet = device::outer(value: erg value); return packet.item + 1; }",
    );

    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("generic nested calls should resolve through the parent facade");
    let plan = build_compilation_plan(&program, &ModuleResolver::new(&fixture.root))
        .expect("qualified generic plan should build");
    emit_program_object(&plan.implementation(), "main")
        .expect("qualified generic call should lower");
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
fn rejects_repeated_imports_with_a_stable_module_diagnostic() {
    let fixture = Fixture::new();
    fixture.write("shared/shared.act", "open types;");
    fixture.write("shared/types.act", "open struct Token { byte: Int, }");
    let program = parse_source(
        "import shared; import shared; verb main() -> Int { erg token = Token { byte: 7, }; return token.byte; }",
    );

    let error = analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect_err("repeated imports must be rejected");
    assert!(
        matches!(error, ModuleError::ImportBindingCollision { ref binding, .. } if binding == "shared")
    );
    assert_eq!(actus::diagnostics::module_diagnostic(&error).code(), "E1115");
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

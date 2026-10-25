use actus::ast::TopLevelDecl;
use actus::lexer::scan;
use actus::parser::{ParseErrorCode, parse};

#[test]
fn parses_extension_free_module_imports() {
    let (tokens, errors) = scan("import driver::gpio;");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("module import should parse");

    let TopLevelDecl::Import(import) = &program.declarations[0] else {
        panic!("expected import declaration");
    };
    assert_eq!(import.path, "driver::gpio");
    assert_eq!(import.alias, None);
    assert!(!import.is_open);
    assert!(import.selected.is_empty());
    assert!(!import.grouped);
    assert_eq!(import.span.start, 0);
}

#[test]
fn parses_grouped_imports_as_normalized_namespace_bindings() {
    let (tokens, errors) = scan("import driver::{i2c, uart};");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("grouped import should parse");

    let paths = program
        .declarations
        .iter()
        .map(|declaration| match declaration {
            TopLevelDecl::Import(import) => {
                assert!(import.alias.is_none());
                assert!(!import.is_open);
                assert!(import.selected.is_empty());
                assert!(import.grouped);
                import.path.as_str()
            }
            _ => panic!("expected import declaration"),
        })
        .collect::<Vec<_>>();
    assert_eq!(paths, ["driver::i2c", "driver::uart"]);
}

#[test]
fn parses_namespace_alias_and_open_items_in_one_import_block() {
    let source = "import { driver::{i2c, uart}, telemetry::uart as telem_uart, open sensors::imu, open config::{MAX_SPEED, MIN_VOLTAGE}, };";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("unified import block should parse");

    let imports = program
        .declarations
        .iter()
        .map(|declaration| match declaration {
            TopLevelDecl::Import(import) => import,
            _ => panic!("expected import declaration"),
        })
        .collect::<Vec<_>>();
    assert_eq!(imports.len(), 6);
    assert_eq!(imports[0].path, "driver::i2c");
    assert!(!imports[0].is_open);
    assert!(imports[0].selected.is_empty());
    assert_eq!(imports[1].path, "driver::uart");
    assert!(!imports[1].is_open);
    assert!(imports[1].selected.is_empty());
    assert_eq!(imports[2].path, "telemetry::uart");
    assert_eq!(imports[2].alias.as_deref(), Some("telem_uart"));
    assert!(!imports[2].is_open);
    assert!(imports[2].selected.is_empty());
    assert_eq!(imports[3].path, "sensors::imu");
    assert!(imports[3].is_open);
    assert_eq!(imports[4].path, "config::MAX_SPEED");
    assert!(imports[4].is_open);
    assert_eq!(imports[5].path, "config::MIN_VOLTAGE");
    assert!(imports[5].is_open);
}

#[test]
fn parses_selected_public_members_without_opening_the_module() {
    let (tokens, errors) = scan("import { module4::submodule { verb1, verb2, }, };");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("selected import should parse");
    let TopLevelDecl::Import(import) = &program.declarations[0] else {
        panic!("expected import declaration");
    };
    assert_eq!(import.path, "module4::submodule");
    assert!(!import.is_open);
    assert_eq!(import.selected, ["verb1", "verb2"]);
}

#[test]
fn keeps_facade_open_as_a_distinct_top_level_declaration() {
    let (tokens, errors) = scan("open reader;");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("facade re-export should parse");
    assert!(matches!(program.declarations[0], TopLevelDecl::OpenSibling(_)));
}

#[test]
fn parses_qualified_call_and_type_names() {
    let source = "verb main() -> telemetry::Packet { return driver::write(value: 1); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("qualified names should parse");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else {
        panic!("expected verb");
    };
    assert_eq!(verb.return_type.as_ref().unwrap().ty.name, "telemetry::Packet");
    let actus::ast::Stmt::Return { value: Some(actus::ast::Expr::Call { callee, .. }), .. } =
        &verb.body.statements[0]
    else {
        panic!("expected qualified call");
    };
    assert_eq!(callee, "driver::write");
}

#[test]
fn parses_qualified_type_names_inside_generic_arguments() {
    let source =
        "verb main() -> Result[manifest::ManifestDescriptor, LogicalBrainError] { return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("qualified generic type names should parse");
}

#[test]
fn parses_imported_qualified_type_names_inside_generic_arguments() {
    let source = "import { open config, ie::persistence::manifest, ie::persistence::cache, }; verb main() -> Result[manifest::ManifestDescriptor, LogicalBrainError] { return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("imported qualified generic type names should parse");
}

#[test]
fn parses_qualified_type_names_in_generic_result_constructors() {
    let source = "verb main() -> Int { return Result[manifest::ManifestDescriptor, LogicalBrainError].Err(0); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("qualified generic result constructors should parse");
}

#[test]
fn rejects_open_imports_outside_the_unified_import_block() {
    let (tokens, errors) = scan("import open sensors::imu;");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("standalone open import should be rejected");
    assert_eq!(error.code, ParseErrorCode::OpenImportRequiresBlock);
}

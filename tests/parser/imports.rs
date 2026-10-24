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
    assert_eq!(imports[1].path, "driver::uart");
    assert_eq!(imports[2].path, "telemetry::uart");
    assert_eq!(imports[2].alias.as_deref(), Some("telem_uart"));
    assert_eq!(imports[3].path, "sensors::imu");
    assert!(imports[3].is_open);
    assert_eq!(imports[4].path, "config::MAX_SPEED");
    assert!(imports[4].is_open);
    assert_eq!(imports[5].path, "config::MIN_VOLTAGE");
    assert!(imports[5].is_open);
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
fn rejects_open_imports_outside_the_unified_import_block() {
    let (tokens, errors) = scan("import open sensors::imu;");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("standalone open import should be rejected");
    assert_eq!(error.code, ParseErrorCode::OpenImportRequiresBlock);
}

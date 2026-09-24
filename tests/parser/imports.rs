use actus::ast::TopLevelDecl;
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn parses_extension_free_module_imports() {
    let (tokens, errors) = scan("import driver::gpio;");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("module import should parse");

    let TopLevelDecl::Import(import) = &program.declarations[0] else {
        panic!("expected import declaration");
    };
    assert_eq!(import.path, "driver::gpio");
    assert_eq!(import.span.start, 0);
}

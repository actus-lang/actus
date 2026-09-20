use actus::ast::{Expr, Role, Stmt, TopLevelDecl};
use actus::lexer::scan;
use actus::parser::{ParseErrorKind, parse};

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

#[test]
fn parses_a_verb_with_roles_and_return_type() {
    let program = parse_source(
        "verb transfer(erg target: File, abs packet: Buffer, dat logger: Logger) -> Int { return target; }",
    );

    let TopLevelDecl::Verb(verb) = &program.declarations[0];
    assert_eq!(verb.name, "transfer");
    assert_eq!(verb.params.len(), 3);
    assert_eq!(verb.params[0].role, Role::Erg);
    assert_eq!(verb.params[1].role, Role::Abs);
    assert_eq!(verb.params[2].role, Role::Dat);
    assert_eq!(verb.return_type.as_ref().map(|ty| ty.name.as_str()), Some("Int"));
    assert!(matches!(verb.body.statements[0], Stmt::Return { .. }));
}

#[test]
fn parses_nested_borrow_and_named_call_arguments() {
    let program = parse_source(
        "verb process() { erg buffer = allocate(10); { abs view = ref buffer; inspect(view, source: view); } }",
    );

    let TopLevelDecl::Verb(verb) = &program.declarations[0];
    let Stmt::OwnerDecl { initializer, .. } = &verb.body.statements[0] else {
        panic!("expected owner declaration");
    };
    assert!(matches!(initializer, Expr::Call { .. }));
    assert!(matches!(verb.body.statements[1], Stmt::Block(_)));
}

#[test]
fn rejects_missing_statement_semicolon() {
    let (tokens, errors) = scan("verb broken() { erg value = 1 }");
    assert!(errors.is_empty());

    let error = parse(tokens).expect_err("missing semicolon must be rejected");
    assert!(matches!(
        error.kind,
        ParseErrorKind::UnexpectedToken { .. } | ParseErrorKind::UnexpectedEndOfInput { .. }
    ));
}

use actus::ast::{Expr, Role, Stmt, TopLevelDecl};
use actus::diagnostics::render_parse_error;
use actus::lexer::scan;
use actus::parser::{ParseErrorCode, ParseErrorKind, parse};

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
fn parses_integer_expression_precedence() {
    let program = parse_source("verb main() -> Int { return 2 + 3 * 4; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0];
    let Stmt::Return { value: Some(Expr::Binary { right, .. }), .. } = &verb.body.statements[0]
    else {
        panic!("expected binary return expression");
    };
    assert!(matches!(right.as_ref(), Expr::Binary { .. }));
}

#[test]
fn rejects_missing_statement_semicolon() {
    let source = include_str!("fixtures/parser/invalid/missing_semicolon.act");
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());

    let error = parse(tokens).expect_err("missing semicolon must be rejected");
    assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
    assert!(matches!(
        error.kind,
        ParseErrorKind::UnexpectedToken { .. } | ParseErrorKind::UnexpectedEndOfInput { .. }
    ));
}

#[test]
fn parses_valid_fixtures() {
    let transfer = parse_source(include_str!("fixtures/parser/valid/transfer.act"));
    let lifecycle = parse_source(include_str!("fixtures/parser/valid/drop.act"));

    assert_eq!(transfer.declarations.len(), 1);
    let TopLevelDecl::Verb(verb) = &lifecycle.declarations[0];
    assert!(matches!(verb.body.statements[1], Stmt::Drop { .. }));
}

#[test]
fn matches_transfer_ast_snapshot() {
    let program = parse_source(include_str!("fixtures/parser/valid/transfer.act"));
    let actual = format!("{program:#?}");
    let expected = include_str!("fixtures/parser/snapshots/transfer.ast.snap");

    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn renders_parser_errors_with_stable_codes_and_locations() {
    let source = include_str!("fixtures/parser/invalid/missing_semicolon.act");
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());

    let error = parse(tokens).expect_err("fixture must fail parsing");
    assert_eq!(
        render_parse_error(source, &error),
        "error[E0003] at 3:1: expected `;`, found RightBrace"
    );
}

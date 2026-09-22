use actus::ast::{Expr, Role, Stmt, StructFieldRole, TopLevelDecl};
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

    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.name, "transfer");
    assert_eq!(verb.params.len(), 3);
    assert_eq!(verb.params[0].role, Role::Erg);
    assert_eq!(verb.params[1].role, Role::Abs);
    assert_eq!(verb.params[2].role, Role::Dat);
    assert_eq!(verb.return_type.as_ref().map(|ty| ty.name.as_str()), Some("Int"));
    assert!(matches!(verb.body.statements[0], Stmt::Return { .. }));
}

#[test]
fn parses_a_struct_with_value_fields() {
    let program = parse_source("struct Point { x: F32, y: F32, }");

    let TopLevelDecl::Struct(definition) = &program.declarations[0] else {
        panic!("expected struct");
    };
    assert_eq!(definition.name, "Point");
    assert_eq!(definition.fields.len(), 2);
    assert!(definition.fields.iter().all(|field| field.role == StructFieldRole::Value));
}

#[test]
fn parses_an_erg_struct_field() {
    let program = parse_source("struct Packet { erg payload: Buffer, sequence: Int, }");

    let TopLevelDecl::Struct(definition) = &program.declarations[0] else {
        panic!("expected struct");
    };
    assert_eq!(definition.fields[0].role, StructFieldRole::Erg);
    assert_eq!(definition.fields[0].name, "payload");
    assert_eq!(definition.fields[1].role, StructFieldRole::Value);
}

#[test]
fn rejects_abs_struct_fields() {
    for (source, role) in [
        ("struct Borrowed { abs view: Buffer, }", actus::lexer::TokenKind::Abs),
        ("struct Moved { dat payload: Buffer, }", actus::lexer::TokenKind::Dat),
    ] {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());

        let error = parse(tokens).expect_err("borrow and move fields must be rejected");
        assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
        assert!(matches!(
            error.kind,
            ParseErrorKind::UnexpectedToken { found, .. } if found == role
        ));
    }
}

#[test]
fn parses_struct_literals_and_field_access() {
    let program = parse_source(
        "struct Point { x: F32, y: F32 } verb main() { erg point = Point { x: 1.0, y: 2.0 }; inspect(point.x); }",
    );

    let TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected verb");
    };
    let Stmt::OwnerDecl { initializer, .. } = &verb.body.statements[0] else {
        panic!("expected owner declaration");
    };
    assert!(matches!(initializer, Expr::StructLit { name, .. } if name == "Point"));
    let Stmt::Expression { expression, .. } = &verb.body.statements[1] else {
        panic!("expected expression statement");
    };
    let Expr::Call { arguments, .. } = expression else { panic!("expected call") };
    assert!(matches!(
        &arguments[0].expression,
        Expr::FieldAccess { field, .. } if field == "x"
    ));
}

#[test]
fn parses_nested_borrow_and_named_call_arguments() {
    let program = parse_source(
        "verb process() { erg buffer = allocate(10); { abs view = ref buffer; inspect(view, source: view); } }",
    );

    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::OwnerDecl { initializer, .. } = &verb.body.statements[0] else {
        panic!("expected owner declaration");
    };
    assert!(matches!(initializer, Expr::Call { .. }));
    assert!(matches!(verb.body.statements[1], Stmt::Block(_)));
}

#[test]
fn parses_integer_expression_precedence() {
    let program = parse_source("verb main() -> Int { return 2 + 3 * 4; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::Binary { right, .. }), .. } = &verb.body.statements[0]
    else {
        panic!("expected binary return expression");
    };
    assert!(matches!(right.as_ref(), Expr::Binary { .. }));
}

#[test]
fn parses_unary_and_grouped_integer_expressions() {
    let program = parse_source("verb main() -> Int { return -(2 + 3); }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::Unary { expression, .. }), .. } = &verb.body.statements[0]
    else {
        panic!("expected unary return expression");
    };
    assert!(matches!(expression.as_ref(), Expr::Grouping { .. }));
}

#[test]
fn rejects_missing_statement_semicolon() {
    let source = include_str!("../fixtures/parser/invalid/missing_semicolon.act");
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
    let transfer = parse_source(include_str!("../fixtures/parser/valid/transfer.act"));
    let lifecycle = parse_source(include_str!("../fixtures/parser/valid/drop.act"));

    assert_eq!(transfer.declarations.len(), 1);
    let TopLevelDecl::Verb(verb) = &lifecycle.declarations[0] else { panic!("expected verb") };
    assert!(matches!(verb.body.statements[1], Stmt::Drop { .. }));
}

#[test]
fn matches_transfer_ast_snapshot() {
    let program = parse_source(include_str!("../fixtures/parser/valid/transfer.act"));
    let actual = format!("{program:#?}");
    let expected = include_str!("../fixtures/parser/snapshots/transfer.ast.snap");

    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn renders_parser_errors_with_stable_codes_and_locations() {
    let source = include_str!("../fixtures/parser/invalid/missing_semicolon.act");
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());

    let error = parse(tokens).expect_err("fixture must fail parsing");
    assert_eq!(
        render_parse_error(source, &error),
        "error[E0003] at 3:1: expected `;`, found RightBrace"
    );
}

use actus::ast::{
    CaseBody, CaseMode, Expr, LiteralPattern, Pattern, Stmt, StructFieldRole, TopLevelDecl,
    VariantPayload,
};
use actus::diagnostics::render_parse_error;
use actus::lexer::scan;
use actus::parser::{ParseErrorCode, ParseErrorKind, parse};

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

#[test]
fn parses_case_variants_literals_and_wildcard_with_spans() {
    let program = parse_source(
        "enum Color { Red, } verb main() -> Int { return case value { Color.Red => 1, true => 2, _ => 0, }; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[1] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::Case { mode, subject, branches, span }), .. } =
        &verb.body.statements[0]
    else {
        panic!("expected case expression");
    };
    assert!(matches!(subject.as_ref(), Expr::Identifier { name, .. } if name == "value"));
    assert_eq!(*mode, CaseMode::Plain);
    assert_eq!(branches.len(), 3);
    assert!(matches!(
        &branches[0].pattern,
        Pattern::Variant { enum_name, variant, payload: VariantPayload::Unit, .. }
            if enum_name == "Color" && variant == "Red"
    ));
    assert!(matches!(
        &branches[1].pattern,
        Pattern::Literal { value: LiteralPattern::Bool(true), .. }
    ));
    assert!(matches!(&branches[2].pattern, Pattern::Wildcard { .. }));
    assert!(matches!(branches[0].body, CaseBody::Expression(_)));
    assert!(branches[0].span.start < branches[0].span.end);
    assert!(span.start < span.end);
}

#[test]
fn parses_case_payload_patterns() {
    let program = parse_source(
        "enum Message { Move(Int, Int), Write { text: String, }, } verb main() { case message { Message.Move(x, y) => x + y, Message.Write(text: t) => 1, }; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[1] else { panic!("expected verb") };
    let Stmt::Expression { expression: Expr::Case { branches, .. }, .. } = &verb.body.statements[0]
    else {
        panic!("expected case statement");
    };
    assert!(matches!(
        &branches[0].pattern,
        Pattern::Variant { payload: VariantPayload::Positional(bindings), .. }
            if bindings.iter().map(|binding| binding.name.as_str()).collect::<Vec<_>>()
                == ["x", "y"]
    ));
    assert!(matches!(
        &branches[1].pattern,
        Pattern::Variant { payload: VariantPayload::Named(fields), .. }
            if fields[0].name == "text" && fields[0].binding.name == "t"
    ));
}

#[test]
fn parses_pattern_guards_without_general_if_statements() {
    let program = parse_source(
        "enum Color { Red, } verb main(abs ready: Bool) { case color { Color.Red if ready => 1, }; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[1] else { panic!("expected verb") };
    let Stmt::Expression { expression: Expr::Case { branches, .. }, .. } = &verb.body.statements[0]
    else {
        panic!("expected case statement");
    };
    assert!(matches!(
        branches[0].guard.as_deref(),
        Some(Expr::Identifier { name, .. }) if name == "ready"
    ));
}

#[test]
fn rejects_case_fallthrough_and_standalone_break() {
    for source in [
        include_str!("../fixtures/parser/invalid/case_fallthrough.act"),
        include_str!("../fixtures/parser/invalid/case_break.act"),
    ] {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        parse(tokens).expect_err("case fallthrough and standalone break must be rejected");
    }
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
fn parses_struct_method_calls() {
    let program = parse_source(
        "struct Point { x: Int, } verb main() -> Int { erg point = Point { x: 1, }; return point.read(); }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected main verb");
    };
    let Stmt::Return { value: Some(Expr::MethodCall { method, .. }), .. } =
        &verb.body.statements[1]
    else {
        panic!("expected method call return");
    };
    assert_eq!(method, "read");
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
        ("struct Instrumented { ins buffer: Buffer, }", actus::lexer::TokenKind::Ins),
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
fn rejects_instrumental_local_bindings() {
    let (tokens, errors) = scan("verb main() { ins buffer = Buffer[1]; }");
    assert!(errors.is_empty());
    parse(tokens).expect_err("ins must not be a local binding role");
}

#[test]
fn rejects_incomplete_call_site_roles() {
    let (tokens, errors) = scan("verb main(erg buffer: Buffer) { consume(buffer: ins); }");
    assert!(errors.is_empty());
    parse(tokens).expect_err("a call-site role must precede an expression");
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
fn parses_field_assignment() {
    let program = parse_source(
        "struct Point { x: Int, } verb main() { erg point = Point { x: 1, }; point.x = 2; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[1] else { panic!("expected verb") };
    assert!(matches!(
        &verb.body.statements[1],
        Stmt::FieldAssignment { field, value: Expr::Integer { value, .. }, .. }
            if field == "x" && value == "2"
    ));
}

#[test]
fn parses_nested_borrow_and_named_call_arguments() {
    let program = parse_source(
        "verb process() { erg buffer = Buffer[10]; { abs view = ref buffer; inspect(view, source: view); } }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::OwnerDecl { initializer, .. } = &verb.body.statements[0] else {
        panic!("expected owner declaration");
    };
    assert!(matches!(initializer, Expr::BufferLiteral { .. }));
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
    let case_program = parse_source(include_str!("../fixtures/parser/valid/case.act"));
    assert_eq!(transfer.declarations.len(), 1);
    assert_eq!(case_program.declarations.len(), 2);
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

#[test]
fn parses_try_operator_as_a_postfix_expression() {
    let program = parse_source(
        "enum IoError { Eof, Failed, } verb read() -> Result[Int, IoError] { return Result[Int, IoError].Ok(1); } verb main() -> Result[Int, IoError] { return read()?; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[2] else { panic!("expected main") };
    let Stmt::Return { value: Some(Expr::Try { .. }), .. } = &verb.body.statements[0] else {
        panic!("expected try expression")
    };
}

#[test]
fn parses_short_result_constructors_and_typed_result_bindings() {
    let program = parse_source(
        "verb main() -> Result[Int, IoError] { erg result: Result[Int, IoError] = Ok(1); return result; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::OwnerDecl { initializer, .. } = &verb.body.statements[0] else {
        panic!("expected owner declaration")
    };
    assert!(matches!(initializer, Expr::Call { callee, .. } if callee == "Ok"));
}

use actus::ast::{
    CaseBody, CaseMode, EnumPayload, Expr, LiteralPattern, MetaAttribute, Pattern, ReturnAccess,
    Role, Stmt, StructFieldRole, TopLevelDecl, VariantPayload,
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
    assert_eq!(verb.return_type.as_ref().map(|ty| ty.ty.name.as_str()), Some("Int"));
    assert_eq!(verb.return_type.as_ref().map(|ty| ty.access), Some(ReturnAccess::Owned));
    assert!(matches!(verb.body.statements[0], Stmt::Return { .. }));
}

#[test]
fn parses_instrumental_parameters_and_call_site_roles() {
    let program = parse_source(
        "verb update(ins buffer: Buffer) { } verb main(erg buffer: Buffer) { update(buffer: ins buffer); }",
    );
    let TopLevelDecl::Verb(update) = &program.declarations[0] else { panic!("expected update") };
    assert_eq!(update.params[0].role, Role::Ins);
    let TopLevelDecl::Verb(main) = &program.declarations[1] else { panic!("expected main") };
    let Stmt::Expression { expression: Expr::Call { arguments, .. }, .. } =
        &main.body.statements[0]
    else {
        panic!("expected call expression")
    };
    assert_eq!(arguments[0].role, Some(Role::Ins));
}

#[test]
fn parses_buffer_literal_construction() {
    let program = parse_source("verb main() { erg buffer = Buffer[16]; }");
    let TopLevelDecl::Verb(main) = &program.declarations[0] else { panic!("expected main") };
    let Stmt::OwnerDecl { initializer: Expr::BufferLiteral { length, .. }, .. } =
        &main.body.statements[0]
    else {
        panic!("expected Buffer literal")
    };
    assert!(matches!(length.as_ref(), Expr::Integer { value, .. } if value == "16"));
}

#[test]
fn parses_meta_test_attribute_without_name_convention() {
    let program = parse_source("meta test\nverb smoke() -> Int { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata, vec![MetaAttribute::Test]);
    assert_eq!(verb.name, "smoke");
}

#[test]
fn parses_generic_parameters_and_type_applications() {
    let program = parse_source(
        "struct Box[T] { item: T, } enum Result[T, E: Error] { Ok(T), Err(E), } verb wrap[T](erg item: T) -> Box[T] { return Box { item: item, }; }",
    );

    let TopLevelDecl::Struct(box_definition) = &program.declarations[0] else {
        panic!("expected generic struct");
    };
    assert_eq!(box_definition.generic_parameters.len(), 1);
    assert_eq!(box_definition.generic_parameters[0].name, "T");
    assert_eq!(box_definition.fields[0].ty.name, "T");

    let TopLevelDecl::Enum(result_definition) = &program.declarations[1] else {
        panic!("expected generic enum");
    };
    assert_eq!(result_definition.generic_parameters.len(), 2);
    assert_eq!(result_definition.generic_parameters[1].bound.as_ref().unwrap().name, "Error");

    let TopLevelDecl::Verb(verb) = &program.declarations[2] else {
        panic!("expected generic verb");
    };
    assert_eq!(verb.generic_parameters[0].name, "T");
    let return_type = verb.return_type.as_ref().expect("generic return type");
    assert_eq!(return_type.ty.name, "Box");
    assert_eq!(return_type.ty.arguments[0].name, "T");
}

#[test]
fn parses_multiple_generic_bounds() {
    let (tokens, errors) = scan("verb write[T: Writer + Serializable](erg item: T) { }");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("multiple generic bounds should parse");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let bounds = &verb.generic_parameters[0].bounds;
    assert_eq!(
        bounds.iter().map(|bound| bound.name.as_str()).collect::<Vec<_>>(),
        ["Writer", "Serializable"]
    );
}

#[test]
fn rejects_duplicate_generic_parameters() {
    let (tokens, errors) = scan("struct Pair[T, T] { first: T, second: T, }");
    assert!(errors.is_empty());

    let error = parse(tokens).expect_err("duplicate generic parameters must be rejected");
    assert_eq!(error.code, ParseErrorCode::DuplicateName);
    assert!(
        matches!(error.kind, ParseErrorKind::DuplicateName { kind, .. } if kind == "generic parameter")
    );
}

#[test]
fn parses_unit_tuple_and_named_enum_variants() {
    let program =
        parse_source("enum Message { Quit, Move(Int, Int), Write { text: String, code: Int, }, }");

    let TopLevelDecl::Enum(definition) = &program.declarations[0] else {
        panic!("expected enum");
    };
    assert_eq!(definition.name, "Message");
    assert!(matches!(definition.variants[0].payload, EnumPayload::Unit));
    assert!(matches!(
        &definition.variants[1].payload,
        EnumPayload::Tuple(types) if types.len() == 2
    ));
    assert!(matches!(
        &definition.variants[2].payload,
        EnumPayload::Struct(fields)
            if fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>()
                == ["text", "code"]
    ));
}

#[test]
fn rejects_duplicate_enum_names_variants_and_payload_fields() {
    let cases = [
        "enum Color { Red, } enum Color { Blue, }",
        "enum Color { Red, Red, }",
        "enum Message { Write { text: String, text: Int, }, }",
    ];

    for source in cases {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let error = parse(tokens).expect_err("duplicate enum names must be rejected");
        assert_eq!(error.code, ParseErrorCode::DuplicateName);
        assert!(matches!(error.kind, ParseErrorKind::DuplicateName { .. }));
    }
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
    assert_eq!(*mode, CaseMode::Abs);
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

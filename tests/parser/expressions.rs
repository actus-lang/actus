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
fn parses_compound_assignment_targets_and_operators() {
    let program = parse_source(
        "verb main() { erg index: u32 = 0u32; index += 1u32; values[0u32] <<= 1u32; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert!(matches!(
        &verb.body.statements[1],
        Stmt::CompoundAssignment { operator: actus::ast::CompoundAssignmentOp::Add, target: actus::ast::CompoundAssignmentTarget::Identifier(name), .. }
            if name == "index"
    ));
    assert!(matches!(
        &verb.body.statements[2],
        Stmt::CompoundAssignment {
            operator: actus::ast::CompoundAssignmentOp::ShiftLeft,
            target: actus::ast::CompoundAssignmentTarget::Index { .. },
            ..
        }
    ));
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
fn rejects_case_fallthrough_but_accepts_loop_control_in_case() {
    let (tokens, errors) = scan(include_str!("../fixtures/parser/invalid/case_fallthrough.act"));
    assert!(errors.is_empty());
    parse(tokens).expect_err("case fallthrough must be rejected");

    parse_source(include_str!("../fixtures/parser/invalid/case_break.act"));
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
fn parses_chained_method_calls_as_left_associative_receivers() {
    let program = parse_source(
        "struct Point { x: Int, } verb main() -> Int { erg point = Point { x: 1, }; return point.first().second(); }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected main verb");
    };
    let Stmt::Return { value: Some(Expr::MethodCall { receiver, method, .. }), .. } =
        &verb.body.statements[1]
    else {
        panic!("expected chained method call");
    };
    assert_eq!(method, "second");
    assert!(matches!(receiver.as_ref(), Expr::MethodCall { method, .. } if method == "first"));
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
fn parses_indirect_struct_reference_fields() {
    let program = parse_source("struct Node { abs next: Node, ins cursor: Node, } ");
    let TopLevelDecl::Struct(definition) = &program.declarations[0] else {
        panic!("expected struct");
    };
    assert_eq!(definition.fields[0].role, StructFieldRole::Abs);
    assert_eq!(definition.fields[1].role, StructFieldRole::Ins);
}

#[test]
fn rejects_dat_struct_fields() {
    let (tokens, errors) = scan("struct Moved { dat payload: Buffer, }");
    assert!(errors.is_empty());
    let error = parse(tokens).expect_err("dat fields must be rejected");
    assert_eq!(error.code, ParseErrorCode::UnexpectedToken);
    assert!(matches!(
        error.kind,
        ParseErrorKind::UnexpectedToken { found, .. } if found == actus::lexer::TokenKind::Dat
    ));
}

#[test]
fn parses_instrumental_arena_reference_bindings() {
    let (tokens, errors) = scan("verb main() { ins node = arena.place(value: node); }");
    assert!(errors.is_empty());
    parse(tokens).expect("ins arena references are local bindings");
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
fn parses_relational_operators_below_arithmetic_precedence() {
    let program = parse_source("verb main() -> Bool { return 1 + 2 < 4 * 2; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::Binary { operator, left, right, .. }), .. } =
        &verb.body.statements[0]
    else {
        panic!("expected relational return expression");
    };
    assert_eq!(*operator, actus::ast::BinaryOp::LessThan);
    assert!(matches!(left.as_ref(), Expr::Binary { operator: actus::ast::BinaryOp::Add, .. }));
    assert!(matches!(
        right.as_ref(),
        Expr::Binary { operator: actus::ast::BinaryOp::Multiply, .. }
    ));
}

#[test]
fn parses_operator_precedence_across_new_binary_families() {
    let program = parse_source("verb main() -> Bool { return 1 + 2 << 1 & 7 == 5 || 1 < 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::Binary { operator, left, .. }), .. } =
        &verb.body.statements[0]
    else {
        panic!("expected logical expression");
    };
    assert_eq!(*operator, actus::ast::BinaryOp::LogicalOr);
    assert!(matches!(left.as_ref(), Expr::Binary { operator: actus::ast::BinaryOp::Equals, .. }));
}

#[test]
fn parses_logical_and_bitwise_unary_operators() {
    let program = parse_source("verb main() -> Bool { return !(1 < 2) && ~(1 ^ 2) != 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::Binary { operator, left, right, .. }), .. } =
        &verb.body.statements[0]
    else {
        panic!("expected logical expression");
    };
    assert_eq!(*operator, actus::ast::BinaryOp::LogicalAnd);
    assert!(matches!(left.as_ref(), Expr::Unary { operator: actus::ast::UnaryOp::LogicalNot, .. }));
    assert!(matches!(
        right.as_ref(),
        Expr::Binary { operator: actus::ast::BinaryOp::NotEquals, .. }
    ));
}

#[test]
fn parses_bounded_array_types_and_indexed_assignments() {
    let program = parse_source(
        "struct Table { cells: Array[Int, 4], } verb main() { erg table = table(); table[1] = 2; inspect(table[1][0]); }",
    );
    let TopLevelDecl::Struct(definition) = &program.declarations[0] else {
        panic!("expected array-containing struct");
    };
    let array_type = &definition.fields[0].ty;
    assert_eq!(array_type.name, "Array");
    assert_eq!(array_type.arguments.len(), 2);
    assert_eq!(array_type.arguments[0].name, "Int");
    assert_eq!(array_type.arguments[1].name, "4");

    let TopLevelDecl::Verb(verb) = &program.declarations[1] else { panic!("expected verb") };
    assert!(matches!(
        &verb.body.statements[1],
        Stmt::IndexAssignment {
            target: Expr::Identifier { name, .. },
            index: Expr::Integer { value, .. },
            value: Expr::Integer { value: assigned, .. },
            ..
        } if name == "table" && value == "1" && assigned == "2"
    ));
    let Stmt::Expression { expression: Expr::Call { arguments, .. }, .. } =
        &verb.body.statements[2]
    else {
        panic!("expected inspection call")
    };
    assert!(matches!(
        &arguments[0].expression,
        Expr::Index { target, index, span }
            if matches!(index.as_ref(), Expr::Integer { value, .. } if value == "0")
                && matches!(target.as_ref(), Expr::Index { target, index, .. }
                    if matches!(target.as_ref(), Expr::Identifier { name, .. } if name == "table")
                        && matches!(index.as_ref(), Expr::Integer { value, .. } if value == "1"))
                && span.start < span.end
    ));
}

#[test]
fn rejects_malformed_bounded_array_capacity() {
    for source in
        ["struct Table { cells: Array[Int], }", "struct Table { cells: Array[Int, 4, 8], }"]
    {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        parse(tokens).expect_err("malformed bounded array must be rejected");
    }
}

#[test]
fn parses_const_generic_array_capacity_for_semantic_validation() {
    let program = parse_source(
        "struct Table[N: Usize] { cells: Array[Int, N], } verb main() -> Int { return 0; }",
    );
    let TopLevelDecl::Struct(definition) = &program.declarations[0] else {
        panic!("expected generic struct")
    };
    assert_eq!(definition.generic_parameters[0].name, "N");
    assert!(matches!(
        definition.generic_parameters[0].kind,
        actus::ast::GenericParamKind::Const { .. }
    ));
    assert_eq!(definition.fields[0].ty.arguments[1].name, "N");
}

#[test]
fn preserves_roles_on_indexed_call_arguments() {
    let program = parse_source(
        "verb consume(ins slot: Buffer) { } verb inspect(abs slot: Buffer) { } verb main() { erg values = Buffer[4]; consume(slot: ins values[0]); inspect(slot: abs values[1]); }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[2] else { panic!("expected main verb") };
    let Stmt::Expression { expression: Expr::Call { arguments, .. }, .. } =
        &verb.body.statements[1]
    else {
        panic!("expected consuming call")
    };
    assert_eq!(arguments[0].role, Some(actus::ast::Role::Ins));
    assert!(matches!(arguments[0].expression, Expr::Index { .. }));

    let Stmt::Expression { expression: Expr::Call { arguments, .. }, .. } =
        &verb.body.statements[2]
    else {
        panic!("expected borrowed call")
    };
    assert_eq!(arguments[0].role, Some(actus::ast::Role::Abs));
    assert!(matches!(arguments[0].expression, Expr::Index { .. }));
}

#[test]
fn rejects_incomplete_index_expression() {
    let (tokens, errors) = scan("verb main() { erg values = Buffer[4]; inspect(values[); }");
    assert!(errors.is_empty());
    parse(tokens).expect_err("an index must contain a complete expression");
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
fn parses_checked_casts_as_postfix_expressions() {
    let program = parse_source("verb main() -> u32 { erg value = 7; return value as u32; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::Cast { expression, target, .. }), .. } =
        &verb.body.statements[1]
    else {
        panic!("expected cast return")
    };
    assert!(matches!(expression.as_ref(), Expr::Identifier { name, .. } if name == "value"));
    assert_eq!(target.name, "u32");
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

use actus::ast::{EnumPayload, Expr, MetaAttribute, ReturnAccess, Role, Stmt, TopLevelDecl};
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
fn attaches_block_docstrings_to_supported_declarations() {
    let program = parse_source(
        "\"\"\"Runs the entry action.\"\"\" verb main() { return 0; } \"\"\"Stores bytes.\"\"\" struct Packet { payload: Buffer, } \"\"\"Defines a reader contract.\"\"\" role Reader { verb read(ins buffer: Buffer); }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.doc.as_deref(), Some("Runs the entry action."));
    let TopLevelDecl::Struct(structure) = &program.declarations[1] else {
        panic!("expected struct")
    };
    assert_eq!(structure.doc.as_deref(), Some("Stores bytes."));
    let TopLevelDecl::Role(role) = &program.declarations[2] else { panic!("expected role") };
    assert_eq!(role.doc.as_deref(), Some("Defines a reader contract."));
}

#[test]
fn docstrings_inside_supported_bodies_are_consumed_as_documentation() {
    let program = parse_source(
        "struct Packet { \"\"\"Payload bytes.\"\"\" payload: Buffer, } enum Status { \"\"\"Ready state.\"\"\" Ready, } verb main() { \"\"\"This statement is documented.\"\"\" return 0; }",
    );
    assert_eq!(program.declarations.len(), 3);
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
    let TopLevelDecl::Verb(main) = &program.declarations[0] else { panic!("expected verb") };
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
        panic!("expected generic struct")
    };
    assert_eq!(box_definition.generic_parameters.len(), 1);
    assert_eq!(box_definition.generic_parameters[0].name, "T");
    assert_eq!(box_definition.fields[0].ty.name, "T");
    let TopLevelDecl::Enum(result_definition) = &program.declarations[1] else {
        panic!("expected generic enum")
    };
    assert_eq!(result_definition.generic_parameters.len(), 2);
    assert_eq!(result_definition.generic_parameters[1].bound.as_ref().unwrap().name, "Error");
    let TopLevelDecl::Verb(verb) = &program.declarations[2] else { panic!("expected verb") };
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
    let TopLevelDecl::Enum(definition) = &program.declarations[0] else { panic!("expected enum") };
    assert_eq!(definition.name, "Message");
    assert!(matches!(definition.variants[0].payload, EnumPayload::Unit));
    assert!(
        matches!(&definition.variants[1].payload, EnumPayload::Tuple(types) if types.len() == 2)
    );
    assert!(
        matches!(&definition.variants[2].payload, EnumPayload::Struct(fields) if fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>() == ["text", "code"])
    );
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

use actus::ast::{
    EnumPayload, Expr, LayoutEndianness, LimitlessScope, MetaAttribute, PackStorage, PrimitiveType,
    ReturnAccess, Role, Stmt, TopLevelDecl, primitive_type,
};
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
fn parses_typed_named_constants_with_source_spans() {
    let program = parse_source(
        "\"\"\"CRC polynomial.\"\"\" const CRC16_POLYNOMIAL: u16 = 4129u16; verb main() { return 0; }",
    );
    let TopLevelDecl::Constant(constant) = &program.declarations[0] else {
        panic!("expected constant")
    };
    assert_eq!(constant.name, "CRC16_POLYNOMIAL");
    assert_eq!(constant.ty.name, "u16");
    assert_eq!(constant.doc.as_deref(), Some("CRC polynomial."));
    assert!(
        matches!(constant.initializer, Expr::Integer { ref value, ref suffix, .. } if value == "4129" && suffix.as_deref() == Some("u16"))
    );
    assert!(constant.span.start < constant.span.end);
}

#[test]
fn parses_target_metadata_into_the_verb_ast() {
    let program = parse_source("meta target(\"unix\") verb platform_action() { return 0; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata, vec![MetaAttribute::Target("unix".to_owned())]);
}

#[test]
fn parses_target_and_test_metadata_together() {
    let program = parse_source(
        "meta target(\"windows\") meta test verb platform_test() -> Int { return 0; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata.len(), 2);
    assert!(verb.metadata.contains(&MetaAttribute::Target("windows".to_owned())));
    assert!(verb.metadata.contains(&MetaAttribute::Test));
}

#[test]
fn parses_limitless_metadata_at_verb_and_file_scope() {
    let program = parse_source(
        "meta limitless(\"file\")\nmeta limitless(\"verb\") verb large() { return 0; }",
    );
    assert_eq!(program.file_metadata, vec![LimitlessScope::File]);
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.metadata, vec![MetaAttribute::Limitless(LimitlessScope::Verb)]);
}

#[test]
fn parses_if_else_and_nested_else_if_expressions() {
    let program = parse_source(
        "verb choose(erg ready: Bool) { if ready { return 1; } else if ready { return 2; } else { return 3; } }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::If { else_branch: Some(actus::ast::IfBranch::ElseIf(nested)), .. } =
        &verb.body.statements[0]
    else {
        panic!("expected statement conditional")
    };
    assert!(matches!(nested.as_ref(), Expr::If { .. }));
}

#[test]
fn keeps_if_expression_distinct_from_statement_if() {
    let program =
        parse_source("verb choose(erg ready: Bool) -> Int { return if ready { 1 } else { 2 }; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::If { .. }), .. } = &verb.body.statements[0] else {
        panic!("expected value-producing if expression")
    };
}

#[test]
fn preserves_a_semicolon_terminated_final_expression_value() {
    let program =
        parse_source("verb choose(erg ready: Bool) -> Int { return if ready { 1; } else { 2; }; }");
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(Expr::If { then_branch, .. }), .. } = &verb.body.statements[0]
    else {
        panic!("expected value-producing if expression")
    };
    let Stmt::Expression { expression: Expr::Integer { span, .. }, span: statement_span } =
        &then_branch.statements[0]
    else {
        panic!("expected final integer expression")
    };
    assert_eq!(span, statement_span);
}

#[test]
fn rejects_unsupported_limitless_scope() {
    let (tokens, errors) = scan("meta limitless(\"project\") verb main() { return 0; }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("unsupported limitless scope should be rejected");
    assert_eq!(error.code, ParseErrorCode::UnsupportedLimitlessScope);
}

#[test]
fn rejects_duplicate_limitless_directives() {
    let (tokens, errors) =
        scan("meta limitless(\"verb\") meta limitless(\"verb\") verb main() { return 0; }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("duplicate limitless metadata should be rejected");
    assert_eq!(error.code, ParseErrorCode::DuplicateMetadata);
}

#[test]
fn rejects_unknown_target_metadata() {
    let (tokens, errors) = scan("meta target(\"plan9\") verb main() { return 0; }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("unknown target should be rejected");
    assert_eq!(error.code, ParseErrorCode::UnsupportedTargetPlatform);
    assert!(matches!(
        error.kind,
        ParseErrorKind::UnsupportedTargetPlatform { name } if name == "plan9"
    ));
}

#[test]
fn rejects_conflicting_target_metadata() {
    let (tokens, errors) =
        scan("meta target(\"unix\") meta target(\"windows\") verb main() { return 0; }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("conflicting targets should be rejected");
    assert_eq!(error.code, ParseErrorCode::ConflictingTargetPlatforms);
    assert!(matches!(
        error.kind,
        ParseErrorKind::ConflictingTargetPlatforms { first, second }
            if first == "unix" && second == "windows"
    ));
}

#[test]
fn rejects_unknown_metadata_attributes() {
    let (tokens, errors) = scan("meta experimental verb main() { return 0; }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("unknown metadata must be rejected");
    assert_eq!(error.code, ParseErrorCode::UnknownMetadata);
    assert!(matches!(
        error.kind,
        ParseErrorKind::UnknownMetadata { name } if name == "experimental"
    ));
}

#[test]
fn rejects_unknown_top_level_keywords_with_a_stable_code() {
    let (tokens, errors) = scan("verbb main() { return 0; }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("unknown declaration keywords must fail");
    assert_eq!(error.code, ParseErrorCode::UnknownKeyword);
    assert!(matches!(
        error.kind,
        ParseErrorKind::UnknownKeyword { name } if name == "verbb"
    ));
}

#[test]
fn rejects_metadata_attached_to_a_non_verb_declaration() {
    let (tokens, errors) = scan("meta test struct Packet { payload: Buffer, }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = parse(tokens).expect_err("metadata must reject invalid declaration targets");
    assert_eq!(error.code, ParseErrorCode::MetadataTargetNotAllowed);
    assert!(matches!(error.kind, ParseErrorKind::MetadataTargetNotAllowed));
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
fn attaches_block_docstrings_to_external_verbs() {
    let program = parse_source(
        "\"\"\"Reads a raw path without transcoding.\"\"\" unsafe extern \"C\" verb host(abs path: Buffer) -> Int;",
    );
    let TopLevelDecl::ExternalVerb(external) = &program.declarations[0] else {
        panic!("expected external verb")
    };
    assert_eq!(external.doc.as_deref(), Some("Reads a raw path without transcoding."));
}

#[test]
fn docstrings_inside_supported_bodies_are_consumed_as_documentation() {
    let program = parse_source(
        "struct Packet { \"\"\"Payload bytes.\"\"\" payload: Buffer, } enum Status { \"\"\"Ready state.\"\"\" Ready, } verb main() { \"\"\"This statement is documented.\"\"\" return 0; }",
    );
    assert_eq!(program.declarations.len(), 3);
}

#[test]
fn preserves_nested_and_performance_documentation_in_the_ast() {
    let program = parse_source(
        r#"
        """Packet storage contract.""" open struct Packet {
            """Owned payload bytes.""" erg payload: Buffer,
        }
        """Status contract.""" open enum Status {
            """Ready variant.""" Ready,
        }
        """Reader contract.""" open role Reader {
            """Read through an ins loan.""" verb read(ins self: Self) -> Int;
        }
        """Reader performance.""" open perform Reader for Packet {
            """Read the packet.""" verb read(ins self: Packet) -> Int { return 0; }
        }
        """Exported sibling.""" open stream;
        "#,
    );
    let TopLevelDecl::Struct(packet) = &program.declarations[0] else { panic!("expected struct") };
    assert_eq!(packet.doc.as_deref(), Some("Packet storage contract."));
    assert_eq!(packet.fields[0].doc.as_deref(), Some("Owned payload bytes."));
    let TopLevelDecl::Enum(status) = &program.declarations[1] else { panic!("expected enum") };
    assert_eq!(status.doc.as_deref(), Some("Status contract."));
    assert_eq!(status.variants[0].doc.as_deref(), Some("Ready variant."));
    let TopLevelDecl::Role(reader) = &program.declarations[2] else { panic!("expected role") };
    assert_eq!(reader.methods[0].doc.as_deref(), Some("Read through an ins loan."));
    let TopLevelDecl::Perform(perform) = &program.declarations[3] else {
        panic!("expected perform")
    };
    assert_eq!(perform.doc.as_deref(), Some("Reader performance."));
    assert_eq!(perform.methods[0].doc.as_deref(), Some("Read the packet."));
    let TopLevelDecl::OpenSibling(sibling) = &program.declarations[4] else {
        panic!("expected sibling")
    };
    assert_eq!(sibling.doc.as_deref(), Some("Exported sibling."));
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
fn preserves_explicit_and_omitted_argument_role_sources() {
    let program = parse_source(
        "verb update(ins value: Int) { } verb main(erg value: Int) { update(value: ins value); update(value: value); }",
    );
    let TopLevelDecl::Verb(main) = &program.declarations[1] else { panic!("expected main") };
    let Stmt::Expression { expression: Expr::Call { arguments: explicit, .. }, .. } =
        &main.body.statements[0]
    else {
        panic!("expected explicit call")
    };
    assert_eq!(explicit[0].role_resolution, actus::ast::ArgumentRoleResolution::Explicit);
    let Stmt::Expression { expression: Expr::Call { arguments: omitted, .. }, .. } =
        &main.body.statements[1]
    else {
        panic!("expected omitted call")
    };
    assert_eq!(omitted[0].role_resolution, actus::ast::ArgumentRoleResolution::Unspecified);
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
fn parses_pack_storage_layout_and_role_qualified_fields() {
    let program = parse_source(
        "pack Control { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs mode: u3 at 1; erg channel: u5 at 4; abs _reserved: u23 at 9 = 0; } }",
    );
    let TopLevelDecl::Pack(pack) = &program.declarations[0] else { panic!("expected pack") };
    assert_eq!(pack.name, "Control");
    assert_eq!(pack.storage.type_name().name, "u32");
    assert_eq!(pack.endianness, LayoutEndianness::Little);
    assert_eq!(pack.fields.len(), 4);
    assert_eq!(pack.fields[0].role, Role::Erg);
    assert_eq!(pack.fields[1].role, Role::Abs);
    assert_eq!(pack.fields[2].offset, 4);
    assert!(
        matches!(pack.fields[3].default_value, Some(Expr::Integer { ref value, .. }) if value == "0")
    );
}

#[test]
fn parses_array_backed_pack_storage_as_a_distinct_frontend_contract() {
    let program = parse_source(
        "pack CacheLine { erg storage: Array[u8, 64]; layout little; fields { erg word_0: u128 at 0; erg word_1: u128 at 128; erg word_2: u128 at 256; erg word_3: u128 at 384; } }",
    );
    let TopLevelDecl::Pack(pack) = &program.declarations[0] else { panic!("expected pack") };
    let PackStorage::ByteArray { element, capacity, .. } = &pack.storage else {
        panic!("expected array-backed pack storage")
    };
    assert_eq!(element.name, "u8");
    assert_eq!(*capacity, 64);
    assert_eq!(pack.storage.byte_capacity(), Some(64));
    assert_eq!(pack.storage.bit_capacity(), Some(512));
    assert_eq!(pack.storage.alignment_bytes(), Some(1));
}

#[test]
fn rejects_pack_unknown_endianness_and_non_access_roles() {
    for source in [
        "pack Control { erg storage: u32; layout middle; fields { erg enabled: u1 at 0; } }",
        "pack Control { erg storage: u32; layout little; fields { dat moved: u1 at 0; } }",
    ] {
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
        assert!(parse(tokens).is_err(), "invalid pack should be rejected");
    }
}

#[test]
fn parses_width_qualified_and_expanded_primitive_types() {
    let program = parse_source(
        "verb primitives(erg bit: u1, erg word: u128, erg signed: i16, erg single: f32, erg double: f64, erg unit: Void) -> Void { return; }",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };

    let names = verb.params.iter().map(|param| param.ty.name.as_str()).collect::<Vec<_>>();
    assert_eq!(names, ["u1", "u128", "i16", "f32", "f64", "Void"]);
    assert_eq!(primitive_type("u1"), Some(PrimitiveType::Integer { signed: false, width: 1 }));
    assert_eq!(primitive_type("i16"), Some(PrimitiveType::Integer { signed: true, width: 16 }));
    assert_eq!(primitive_type("f32"), Some(PrimitiveType::Float { width: 32 }));
    assert_eq!(primitive_type("f64"), Some(PrimitiveType::Float { width: 64 }));
    assert_eq!(primitive_type("Void"), Some(PrimitiveType::Void));
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

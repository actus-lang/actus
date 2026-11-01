use actus::ffi::{
    CAbiError, CAbiLayout, CAbiOwnership, CAbiParameterPassing, CAbiReturnOwnership, CAbiTarget,
    CAbiType, CallingConvention, c_abi_external_signature, c_abi_external_signature_in_program,
    c_abi_signature, c_abi_signature_in_program, c_abi_signature_in_program_for_target,
};
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

fn parse_verb(source: &str) -> actus::ast::VerbDecl {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    match program.declarations.into_iter().next().expect("verb should exist") {
        actus::ast::TopLevelDecl::Verb(verb) => verb,
        actus::ast::TopLevelDecl::ExternalVerb(_) | actus::ast::TopLevelDecl::Constant(_) => {
            panic!("expected regular verb")
        }
        actus::ast::TopLevelDecl::Struct(_)
        | actus::ast::TopLevelDecl::Pack(_)
        | actus::ast::TopLevelDecl::Serialize(_)
        | actus::ast::TopLevelDecl::Enum(_)
        | actus::ast::TopLevelDecl::Role(_)
        | actus::ast::TopLevelDecl::Perform(_)
        | actus::ast::TopLevelDecl::OpenSibling(_)
        | actus::ast::TopLevelDecl::Import(_) => {
            panic!("expected regular verb")
        }
    }
}

fn parse_program(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

#[test]
fn maps_actus_roles_and_types_to_a_c_abi_signature() {
    let verb = parse_verb(
        "verb exchange(erg target: Buffer, abs view: Buffer, erg count: Int) -> Int { return 0; }",
    );
    let signature = c_abi_signature(&verb).expect("signature should map");

    assert_eq!(signature.calling_convention, CallingConvention::C);
    assert_eq!(signature.return_type, CAbiType::Int32);
    assert_eq!(signature.return_ownership, CAbiReturnOwnership::Value);
    assert_eq!(signature.parameters[0].ty, CAbiType::OpaquePointer);
    assert_eq!(signature.parameters[0].ownership, CAbiOwnership::Exclusive);
    assert_eq!(signature.parameters[1].ownership, CAbiOwnership::SharedBorrow);
    assert_eq!(signature.parameters[2].ownership, CAbiOwnership::Exclusive);
}

#[test]
fn maps_instrumental_buffers_to_exclusive_c_pointers() {
    let verb = parse_verb("verb append(ins buffer: Buffer) -> Int { return 0; }");
    let signature = c_abi_signature(&verb).expect("instrumental buffer should map");

    assert_eq!(signature.parameters[0].ty, CAbiType::OpaquePointer);
    assert_eq!(signature.parameters[0].ownership, CAbiOwnership::Exclusive);
}

#[test]
fn rejects_implicit_returns_at_the_c_abi_boundary() {
    let verb = parse_verb("verb exchange(erg target: Buffer) { drop(target); }");
    let error = c_abi_signature(&verb).expect_err("C exports require explicit return types");

    assert!(matches!(error, CAbiError::MissingReturnType { verb } if verb == "exchange"));
}

#[test]
fn rejects_registered_types_without_a_c_abi_mapping() {
    let verb = parse_verb("verb exchange(erg items: Array) -> Int { return 0; }");
    let error = c_abi_signature(&verb).expect_err("Array has no initial C ABI mapping");

    assert!(matches!(error, CAbiError::UnsupportedType { name } if name == "Array"));
}

#[test]
fn maps_c_abi_types_to_target_layouts() {
    let target = CAbiTarget::new(8).expect("64-bit pointer width should be accepted");

    assert_eq!(CAbiType::Int32.c_name(), "int32_t");
    assert_eq!(CAbiType::OpaquePointer.c_name(), "void*");
    assert_eq!(target.layout(CAbiType::Int32), CAbiLayout { size: 4, alignment: 4 });
    assert_eq!(target.layout(CAbiType::OpaquePointer), CAbiLayout { size: 8, alignment: 8 });
    assert!(CAbiTarget::new(16).is_none());
}

#[test]
fn exposes_stable_calling_convention_name() {
    assert_eq!(CallingConvention::C.name(), "C");
}

#[test]
fn models_external_c_declarations_without_a_definition_body() {
    let (tokens, errors) = scan(
        "unsafe extern \"C\" verb write(dat buffer: Buffer) -> Int; verb main() -> Int { return 0; }",
    );
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    analyze(&program).expect("external declaration should pass semantic analysis");
    let external = match &program.declarations[0] {
        actus::ast::TopLevelDecl::ExternalVerb(declaration) => declaration,
        _ => panic!("expected external declaration"),
    };
    let signature = c_abi_external_signature(external).expect("C declaration should map");

    assert_eq!(signature.name, "write");
    assert_eq!(signature.parameters[0].ownership, CAbiOwnership::Consumed);
}

#[test]
fn validates_ins_ownership_before_an_external_c_call() {
    let source = "unsafe extern \"C\" verb mutate(ins buffer: Buffer) -> Int; verb main() -> Int { erg buffer = Buffer[4]; return mutate(buffer: ins buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");

    analyze(&program).expect("ins ownership should be valid at the C boundary");
}

#[test]
fn rejects_ffi_aliasing_before_native_lowering() {
    let source = "unsafe extern \"C\" verb merge(ins target: Buffer, abs view: Buffer) -> Int; verb main() -> Int { erg buffer = Buffer[4]; return merge(target: ins buffer, view: abs buffer); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("aliased FFI arguments must be rejected");

    assert!(matches!(error.kind, actus::semantic::SemanticErrorKind::ExclusiveLoanAlias { .. }));
}

#[test]
fn marks_buffer_returns_as_owned_resources() {
    let verb = parse_verb("verb allocate_buffer() -> Buffer { return Buffer[4]; }");
    let signature = c_abi_signature(&verb).expect("Buffer return should map");

    assert_eq!(signature.return_ownership, CAbiReturnOwnership::OwnedResource);
}

#[test]
fn marks_abs_buffer_returns_as_borrowed_pointers() {
    let verb = parse_verb("verb view(abs source: Buffer) -> abs Buffer { return source; }");
    let signature = c_abi_signature(&verb).expect("borrowed buffer return should map");

    assert_eq!(signature.return_ownership, CAbiReturnOwnership::Borrowed);
}

#[test]
fn rejects_abs_scalar_returns_at_the_c_abi_boundary() {
    let verb = parse_verb("verb view(erg source: Int) -> abs Int { return source; }");
    let error = c_abi_signature(&verb).expect_err("scalar abs return must be rejected");

    assert!(matches!(error, CAbiError::InvalidReturnOwnership { access, ty }
        if access == "abs" && ty == "Int"));
}

#[test]
fn rejects_non_c_external_declarations() {
    let (tokens, errors) = scan("extern \"Rust\" verb write() -> Int;");
    assert!(errors.is_empty());
    let error = parse(tokens).expect_err("unknown ABI must be rejected by the parser");

    assert!(
        matches!(error.kind, actus::parser::ParseErrorKind::UnexpectedToken { expected, .. } if expected.contains("currently `\"C\"`"))
    );
}

#[test]
fn rejects_external_declarations_without_an_unsafe_boundary() {
    let (tokens, errors) = scan("extern \"C\" verb write(dat buffer: Buffer) -> Int;");
    assert!(errors.is_empty());
    let program = parse(tokens).expect("external declaration should parse");
    let external = match &program.declarations[0] {
        actus::ast::TopLevelDecl::ExternalVerb(declaration) => declaration,
        _ => panic!("expected external declaration"),
    };
    let error = c_abi_external_signature(external).expect_err("unsafe boundary is required");
    assert!(matches!(error, CAbiError::MissingUnsafeBoundary { verb } if verb == "write"));
}

#[test]
fn rejects_primitive_consuming_and_borrowed_parameters() {
    let verb = parse_verb("verb exchange(abs view: Int, dat count: Int) -> Int { return 0; }");
    let error = c_abi_signature(&verb).expect_err("primitive ownership roles need no C mapping");
    assert!(matches!(error, CAbiError::InvalidOwnership { parameter, role, ty }
        if parameter == "view" && role == "abs" && ty == "Int"));
}

#[test]
fn maps_repr_c_aggregates_with_layout_and_explicit_pointer_passing() {
    let program = parse_program(
        "meta repr(C) struct Point { x: i32, y: i32, } verb move_point(erg point: Point) -> Point { return point; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let signature =
        c_abi_signature_in_program(&program, verb).expect("repr(C) aggregate should map");
    let CAbiType::Aggregate(aggregate) = &signature.parameters[0].ty else {
        panic!("expected aggregate parameter")
    };
    assert_eq!(aggregate.name, "Point");
    assert_eq!(aggregate.layout, CAbiLayout { size: 8, alignment: 4 });
    assert_eq!(aggregate.fields[1].offset, 4);
    assert_eq!(signature.parameters[0].passing, CAbiParameterPassing::Pointer);
    assert_eq!(signature.return_ownership, CAbiReturnOwnership::CallerAllocated);
}

#[test]
fn maps_repr_c_aggregates_on_external_c_declarations() {
    let program = parse_program(
        "meta repr(C) struct Point { x: i32, y: i32, } unsafe extern \"C\" verb consume(erg point: Point) -> Int;",
    );
    let actus::ast::TopLevelDecl::ExternalVerb(declaration) = &program.declarations[1] else {
        panic!("expected external verb")
    };
    let signature = c_abi_external_signature_in_program(&program, declaration)
        .expect("repr(C) external aggregate should map");
    assert!(matches!(signature.parameters[0].ty, CAbiType::Aggregate(_)));
    assert_eq!(signature.parameters[0].passing, CAbiParameterPassing::Pointer);
}

#[test]
fn rejects_unannotated_aggregates_at_the_program_aware_ffi_boundary() {
    let program = parse_program(
        "struct Point { x: i32, y: i32, } verb consume(erg point: Point) -> Int { return 0; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let error = c_abi_signature_in_program(&program, verb)
        .expect_err("unannotated aggregate must not cross the C ABI");
    assert!(matches!(error, CAbiError::AggregateRequiresReprC { name } if name == "Point"));
}

#[test]
fn maps_all_checked_repr_c_scalars_arrays_and_target_sized_values() {
    let program = parse_program(
        "meta repr(C) struct Wire { wide: i64, fraction: f32, flag: Bool, word: Usize, bytes: Array[u16, 2], } verb inspect(erg wire: Wire) -> Int { return 0; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let signature = c_abi_signature_in_program_for_target(
        CAbiTarget::new(4).expect("32-bit target should be accepted"),
        &program,
        verb,
    )
    .expect("all checked repr(C) field types should map");
    let CAbiType::Aggregate(aggregate) = &signature.parameters[0].ty else {
        panic!("expected aggregate parameter")
    };

    assert_eq!(aggregate.fields[0].layout, CAbiLayout { size: 8, alignment: 4 });
    assert_eq!(aggregate.fields[1].layout, CAbiLayout { size: 4, alignment: 4 });
    assert_eq!(aggregate.fields[2].layout, CAbiLayout { size: 1, alignment: 1 });
    assert_eq!(aggregate.fields[3].layout, CAbiLayout { size: 4, alignment: 4 });
    assert_eq!(aggregate.fields[4].layout, CAbiLayout { size: 4, alignment: 2 });
    assert_eq!(aggregate.layout, CAbiLayout { size: 24, alignment: 4 });
}

#[test]
fn maps_repr_c_packed_fields_without_padding() {
    let program = parse_program(
        "meta repr(C, packed) struct Wire { prefix: u8, value: u32, tail: u16, } verb inspect(erg wire: Wire) -> Int { return 0; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let signature =
        c_abi_signature_in_program(&program, verb).expect("packed repr(C) aggregate should map");
    let CAbiType::Aggregate(aggregate) = &signature.parameters[0].ty else {
        panic!("expected aggregate parameter")
    };
    assert!(aggregate.packed);
    assert_eq!(aggregate.layout, CAbiLayout { size: 7, alignment: 1 });
    assert_eq!(aggregate.fields[0].offset, 0);
    assert_eq!(aggregate.fields[1].offset, 1);
    assert_eq!(aggregate.fields[2].offset, 5);
}

#[test]
fn maps_repr_transparent_as_the_underlying_c_abi_value() {
    let program = parse_program(
        "meta repr(transparent) struct UserId { value: u64, } verb inspect(erg id: UserId) -> UserId { return id; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let signature = c_abi_signature_in_program(&program, verb)
        .expect("transparent aggregate should map to its underlying value");
    assert_eq!(signature.parameters[0].ty, CAbiType::Integer { signed: false, width: 64 });
    assert_eq!(signature.parameters[0].passing, CAbiParameterPassing::Direct);
    assert_eq!(signature.return_type, CAbiType::Integer { signed: false, width: 64 });
    assert_eq!(signature.return_ownership, CAbiReturnOwnership::Value);
}

#[test]
fn maps_nested_transparent_ffi_values_without_an_extra_aggregate_layer() {
    let program = parse_program(
        "meta repr(transparent) struct UserId { value: u64, } meta repr(transparent) struct RequestId { value: UserId, } verb inspect(erg id: RequestId) -> Int { return 0; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[2] else {
        panic!("expected ordinary verb")
    };
    let signature = c_abi_signature_in_program(&program, verb)
        .expect("nested transparent aggregate should map recursively");
    assert_eq!(signature.parameters[0].ty, CAbiType::Integer { signed: false, width: 64 });
    assert_eq!(signature.parameters[0].passing, CAbiParameterPassing::Direct);
}

#[test]
fn rejects_combined_transparent_ffi_representation_before_mapping() {
    let program = parse_program(
        "meta repr(C, transparent) struct UserId { value: u64, } verb inspect(erg id: UserId) -> Int { return 0; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let error = c_abi_signature_in_program(&program, verb)
        .expect_err("combined transparent representations must be rejected");
    assert!(
        matches!(error, CAbiError::TransparentCombinedRepresentation { name } if name == "UserId")
    );
}

#[test]
fn maps_repr_c_explicit_alignment_into_the_aggregate_layout() {
    let program = parse_program(
        "meta repr(C, align(8u32)) struct Aligned { value: u8, } verb inspect(erg value: Aligned) -> Int { return 0; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let signature = c_abi_signature_in_program(&program, verb)
        .expect("explicitly aligned repr(C) aggregate should map");
    let CAbiType::Aggregate(aggregate) = &signature.parameters[0].ty else {
        panic!("expected aggregate parameter")
    };
    assert_eq!(aggregate.layout, CAbiLayout { size: 8, alignment: 8 });
    assert_eq!(aggregate.fields[0].offset, 0);
}

#[test]
fn maps_repr_c_payload_enum_as_a_checked_tagged_union() {
    let program = parse_program(
        "meta repr(C) enum Message { Byte(u8), Pair { code: u16, value: u32, }, Empty, } verb inspect(erg message: Message) -> Int { return 0; }",
    );
    let actus::ast::TopLevelDecl::Verb(verb) = &program.declarations[1] else {
        panic!("expected ordinary verb")
    };
    let signature = c_abi_signature_in_program(&program, verb)
        .expect("payload enum should map to a tagged union");
    let CAbiType::TaggedUnion(union) = &signature.parameters[0].ty else {
        panic!("expected tagged-union parameter")
    };
    assert_eq!(union.tag, CAbiLayout { size: 4, alignment: 4 });
    assert_eq!(union.payload_offset, 4);
    assert_eq!(union.layout, CAbiLayout { size: 12, alignment: 4 });
    assert_eq!(union.variants[0].discriminant, 0);
    assert_eq!(union.variants[1].discriminant, 1);
    assert_eq!(union.variants[2].discriminant, 2);
    assert_eq!(signature.parameters[0].passing, CAbiParameterPassing::Pointer);
}

use actus::ast::{
    BuiltinType, IntrinsicKind, PrimitiveType, RegistryStatus, lookup_builtin_type,
    lookup_call_intrinsic, lookup_intrinsic, primitive_type,
};
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, SemanticType, TypeRegistry, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program)
}

#[test]
fn intrinsic_registry_defines_source_contracts() {
    assert!(lookup_intrinsic("allocate").is_none());
    assert_eq!(IntrinsicKind::Append.spec().parameters, &["handle", "byte"]);
    assert_eq!(IntrinsicKind::Crc32.spec().parameters, &["buffer", "start", "end"]);
    assert_eq!(
        IntrinsicKind::Crc32Matches.spec().parameters,
        &["buffer", "start", "end", "expected"]
    );
    assert_eq!(
        IntrinsicKind::ValidateFixedFrame.spec().parameters,
        &[
            "buffer",
            "little",
            "version_offset",
            "expected_version",
            "payload_offset",
            "payload_length",
            "checksum_start",
            "checksum_end",
            "checksum_offset"
        ]
    );
    assert_eq!(IntrinsicKind::Copy.spec().parameters, &["value"]);
    assert_eq!(IntrinsicKind::Print.spec().parameters, &["value"]);
    assert_eq!(IntrinsicKind::Drop.spec().status, RegistryStatus::Active);
    assert_eq!(lookup_intrinsic("drop"), Some(IntrinsicKind::Drop));
    assert!(lookup_call_intrinsic("drop").is_none());
    assert_eq!(lookup_call_intrinsic("print"), Some(IntrinsicKind::Print));
    assert_eq!(lookup_call_intrinsic("copy"), Some(IntrinsicKind::Copy));
    assert_eq!(lookup_call_intrinsic("crc32"), Some(IntrinsicKind::Crc32));
    assert_eq!(lookup_call_intrinsic("crc32_matches"), Some(IntrinsicKind::Crc32Matches));
    assert_eq!(
        lookup_call_intrinsic("validate_fixed_frame"),
        Some(IntrinsicKind::ValidateFixedFrame)
    );
    assert!(lookup_intrinsic("user_function").is_none());
}

#[test]
fn compiler_provides_serialization_error_domain() {
    analyze_source(
        "verb main() -> Result[Int, SerializationError] { return Result[Int, SerializationError].Err(SerializationError.InvalidChecksum); }",
    )
    .expect("SerializationError should be available as a compiler-provided enum");
}

#[test]
fn accepts_compiler_provided_buffer_length() {
    analyze_source(
        "verb main() -> Int { erg buffer: Buffer = Buffer[4]; return buffer_length(buffer: abs buffer); }",
    )
    .expect("buffer_length should accept a read-only buffer");
}

#[test]
fn accepts_explicit_copy_of_integer_and_boolean_scalars() {
    analyze_source(
        "verb main() -> Int { erg value: u32 = 41u32; erg repeated: u32 = copy(value: abs value); erg flag: Bool = true; erg repeated_flag: Bool = copy(value: abs flag); if repeated_flag { return repeated as Int; } return 0; }",
    )
    .expect("explicit copy should preserve eligible scalar values");
}

#[test]
fn accepts_bounded_for_ranges_and_rejects_non_integer_bindings() {
    analyze_source(
        "verb main() { for erg index: u32 in 0u32 .. 4u32 { if index == 2u32 { break; } } }",
    )
    .expect("bounded integer range should be valid");
    let error = analyze_source("verb main() { for abs index: u32 in 0u32 .. 4u32 { } }")
        .expect_err("range bindings must be erg");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidArgumentRole { callee, .. } if callee == "for")
    );
    let unbounded = analyze_source("verb main() { for erg index: u32 in 1u32 { } }")
        .expect_err("scalar sources must not become unbounded iterators");
    assert!(
        matches!(unbounded.kind, SemanticErrorKind::TypeMismatch { callee, parameter, .. } if callee == "for" && parameter == "array source")
    );
    analyze_source("verb main() { for erg index: u8 in 0u8 .. 255u8 { } }")
        .expect("the maximum representable endpoint is a valid half-open bound");
    analyze_source("verb main() { for erg index: u8 in 0u8 .. 256u16 { } }")
        .expect_err("range bounds must use the declared integer type");
}

#[test]
fn rejects_copy_without_an_explicit_read_only_role() {
    let error = analyze_source(
        "verb main() { erg value: u32 = 41u32; erg repeated: u32 = copy(value: value); }",
    )
    .expect_err("copy must require an explicit abs view");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::InvalidArgumentRole { callee, parameter }
            if callee == "copy" && parameter == "value"
    ));
}

#[test]
fn rejects_copy_of_owned_aggregate_values() {
    let error = analyze_source(
        "verb main() { erg buffer: Buffer = Buffer[4]; erg repeated = copy(value: abs buffer); drop(buffer); drop(repeated); }",
    )
    .expect_err("copy must reject cleanup-bearing aggregates");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter }
            if callee == "copy" && parameter == "value"
    ));
}

#[test]
fn rejects_copy_after_an_owning_transfer() {
    let error = analyze_source(
        "verb consume(dat value: u32) { } verb main() { erg value: u32 = 41u32; consume(value: dat value); erg repeated: u32 = copy(value: abs value); }",
    )
    .expect_err("an owning transfer must leave the source unavailable");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::UseAfterMove { name } if name == "value"
    ));
}

#[test]
fn scalar_copy_preserves_the_owner_across_branch_joins() {
    analyze_source(
        "verb main() -> Int { erg value: u32 = 41u32; if true { erg left: u32 = copy(value: abs value); } else { erg right: u32 = copy(value: abs value); } return value as Int + 1; }",
    )
    .expect("read-only scalar reuse must restore the owner at a branch join");
}

#[test]
fn validates_print_arguments() {
    analyze_source("verb main() { print(42); }").expect("integer output should be valid");
    analyze_source("verb main() { print(\"text\"); }")
        .expect("string output should use the same print intrinsic");
    analyze_source("verb main() { erg text = \"text\"; print(text); }")
        .expect("string bindings should use the same print intrinsic");
}

#[test]
fn validates_typed_constants_and_resolves_constant_references() {
    analyze_source("const FRAME_HEADER: u16 = 12u16; verb main() -> u16 { return FRAME_HEADER; }")
        .expect("typed constant reference should be valid");
}

#[test]
fn validates_named_pack_offsets_and_rejects_runtime_offsets() {
    analyze_source(
        "const OFFSET: u16 = 8u16; pack Frame { erg storage: Array[u8, 2]; layout little; fields { erg prefix: u8 at 0; erg marker: u8 at OFFSET; } } verb main() { return; }",
    )
    .expect("named pack offsets should use compile-time constants");
    let error = analyze_source(
        "verb runtime_offset() -> u16 { return 8u16; } pack Frame { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at runtime_offset; } } verb main() { return; }",
    )
    .expect_err("runtime offset names must be rejected");
    assert!(
        matches!(error.kind, SemanticErrorKind::ConstantRuntimeDependency { name } if name == "runtime_offset")
    );
}

#[test]
fn validates_boolean_literals_and_rejects_integer_bindings() {
    analyze_source(
        "const DEFAULT_LINKED: Bool = false; verb main() -> Bool { erg linked: Bool = false; return linked; }",
    )
    .expect("Boolean literals should be valid values and constants");
    let error = analyze_source("verb main() { erg count: Int = true; }")
        .expect_err("Boolean literals must not coerce to integers");
    assert!(matches!(error.kind, SemanticErrorKind::BindingTypeMismatch { .. }));
}

#[test]
fn rejects_duplicate_and_overflowing_constants() {
    let duplicate =
        analyze_source("const LIMIT: u8 = 4u8; const LIMIT: u8 = 8u8; verb main() { return; }")
            .expect_err("duplicate constants must be rejected");
    assert!(
        matches!(duplicate.kind, SemanticErrorKind::DuplicateConstantName { name } if name == "LIMIT")
    );

    analyze_source("const LIMIT: u8 = 256u16; verb main() { return; }")
        .expect_err("constant overflow must be rejected");

    let cycle = analyze_source(
        "const FIRST: u8 = SECOND; const SECOND: u8 = FIRST; verb main() { return; }",
    )
    .expect_err("constant cycles must be rejected");
    assert!(matches!(cycle.kind, SemanticErrorKind::ConstantCycle { .. }));
}

#[test]
fn accepts_forward_referenced_constant_expressions() {
    analyze_source(
        "const ANSWER: Int = OFFSET + 1; const OFFSET: Int = 41; verb main() -> Int { return ANSWER; }",
    )
    .expect("constant expressions may reference later constants");
}

#[test]
fn rejects_runtime_dependencies_in_constant_initializers() {
    let error = analyze_source(
        "verb runtime_value() -> Int { return 41; } const ANSWER: Int = runtime_value(); verb main() { return; }",
    )
    .expect_err("constants must not depend on runtime calls");
    assert!(
        matches!(error.kind, SemanticErrorKind::ConstantRuntimeDependency { name } if name == "ANSWER")
    );
}

#[test]
fn accepts_statement_if_control_flow_and_diverging_expression_branches() {
    analyze_source("verb main() -> Int { loop { if 1 == 1 { break; } } return 42; }")
        .expect("statement if should participate in loop control flow");
    analyze_source(
        "verb choose(erg ready: Bool) -> Int { return if ready { return 41; } else { 42 }; }",
    )
    .expect("a diverging expression branch should not require a matching value");
}

#[test]
fn accepts_unsuffixed_integer_literals_in_known_integer_operations() {
    analyze_source("verb main() -> u32 { erg index: u32 = 0; index += 1; return index + 1; }")
        .expect("known integer operands should direct unsuffixed literals");
}

#[test]
fn rejects_out_of_range_unsuffixed_integer_literals_in_known_integer_operations() {
    let error =
        analyze_source("verb main() -> u8 { erg value: u8 = 1; value += 256; return value; }")
            .expect_err("type-directed literals must retain range checking");
    assert!(matches!(error.kind, SemanticErrorKind::NumericLiteralOutOfRange { .. }));
}

#[test]
fn validates_relational_operand_families() {
    analyze_source("verb main() -> Bool { return 1 < 2; }")
        .expect("matching Int operands should compare");
    analyze_source(
        "verb main() -> Bool { erg left: u8 = 1; erg right: u8 = 2; return left <= right; }",
    )
    .expect("matching unsigned operands should compare");
    analyze_source(
        "verb main() -> Bool { erg left: u8 = 1; erg right: i8 = 2; return left < right; }",
    )
    .expect_err("signed and unsigned operands must not mix");
    analyze_source("verb main() -> Bool { return 1 < 2.0; }")
        .expect_err("integer and float operands must not mix");
}

#[test]
fn validates_adr44_operator_operand_families() {
    analyze_source("verb main() -> Bool { return 1 == 1; }")
        .expect("integer equality should be valid");
    analyze_source("verb main() -> Bool { return (1 < 2) != (2 < 3); }")
        .expect("Bool equality should be valid");
    analyze_source("verb main() -> Int { return 7 % 3; }")
        .expect("integer remainder should be valid");
    analyze_source("verb main() -> Bool { return !(1 < 2) || (2 == 3); }")
        .expect("Bool logical operators should be valid");
    analyze_source(
        "verb main() -> u8 { erg left: u8 = 7; erg right: u8 = 3; return left & right; }",
    )
    .expect("integer bitwise operators should be valid");
    analyze_source(
        "verb main() -> u8 { erg left: u8 = 7; erg count: u8 = 1; return left << count; }",
    )
    .expect("unsigned shift counts should be valid");
}

#[test]
fn rejects_adr44_invalid_operator_operands() {
    for source in [
        "verb main() -> Bool { erg left: u8 = 1; erg right: i8 = 1; return left == right; }",
        "verb main() -> Int { return 1.5 % 1.0; }",
        "verb main() -> Bool { return 1 && (2 < 3); }",
        "verb main() -> Int { return 1.5 & 2.5; }",
        "verb main() -> u8 { erg left: u8 = 1; erg count: i8 = 1; return left << count; }",
        "verb main() -> Bool { return !1; }",
    ] {
        analyze_source(source).expect_err("invalid ADR-0044 operands must be rejected");
    }
}

#[test]
fn rejects_statically_provable_invalid_integer_operations() {
    let remainder = analyze_source("verb main() -> Int { return 7 % 0; }")
        .expect_err("constant zero remainder must be rejected");
    assert!(matches!(remainder.kind, SemanticErrorKind::ConstantRemainderByZero));

    let shift =
        analyze_source("verb main() -> u8 { erg input: u8 = 1; return input << (8 as u8); }")
            .expect_err("constant shift count equal to width must be rejected");
    assert!(matches!(
        shift.kind,
        SemanticErrorKind::ConstantShiftCountOutOfRange { count, width }
            if count == "8" && width == 8
    ));
}

#[test]
fn builtin_type_registry_defines_supported_types() {
    assert_eq!(lookup_builtin_type("Int"), Some(BuiltinType::Int));
    assert_eq!(lookup_builtin_type("String"), Some(BuiltinType::String));
    assert_eq!(BuiltinType::Buffer.spec().name, "Buffer");
    assert_eq!(BuiltinType::Buffer.spec().status, RegistryStatus::Active);
    assert_eq!(lookup_builtin_type("Array"), Some(BuiltinType::Array));
    assert_eq!(lookup_builtin_type("Map"), Some(BuiltinType::Map));
}

#[test]
fn primitive_type_registry_preserves_width_and_signedness() {
    assert_eq!(primitive_type("u3"), Some(PrimitiveType::Integer { signed: false, width: 3 }));
    assert_eq!(primitive_type("i8"), Some(PrimitiveType::Integer { signed: true, width: 8 }));
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.resolve("f32"),
        Some(SemanticType::Primitive(PrimitiveType::Float { width: 32 }))
    );
    assert_eq!(registry.resolve("Void"), Some(SemanticType::Primitive(PrimitiveType::Void)));
}

#[test]
fn accepts_primitive_integer_boundary_literals() {
    analyze_source(
        "verb main() -> Void { erg a: u3 = 7; erg b: u8 = 255; erg c: i8 = -128; erg d: i8 = 127; erg e: i8 = 0x7F; return; }",
    )
    .expect("boundary literals should be accepted");
}

#[test]
fn accepts_typed_integer_literals_and_preserves_their_type() {
    analyze_source(
        "verb main() -> Void { erg index = 1u32; erg byte: u8 = 0u8; erg signed: i32 = -1i32; return; }",
    )
    .expect("typed integer literals should infer and preserve their primitive types");
}

#[test]
fn rejects_typed_integer_literal_mismatches_and_overflow() {
    for source in [
        "verb main() { erg byte: u8 = 1u32; }",
        "verb main() { erg byte = 256u8; }",
        "verb main() { erg byte = -1u8; }",
    ] {
        assert!(
            analyze_source(source).is_err(),
            "typed integer literal must match and fit its suffix: {source}"
        );
    }
}

#[test]
fn rejects_primitive_integer_overflow_and_unsigned_underflow() {
    for source in [
        "verb main() { erg value: u3 = 8; }",
        "verb main() { erg value: u1 = 2; }",
        "verb main() { erg value: u8 = 256; }",
        "verb main() { erg value: u8 = -1; }",
    ] {
        let error = analyze_source(source).expect_err("out-of-range literal must be rejected");
        assert!(matches!(error.kind, SemanticErrorKind::NumericLiteralOutOfRange { .. }));
    }
}

#[test]
fn rejects_mismatched_primitive_calls_and_assignments() {
    let call_error = analyze_source(
        "verb consume(erg sample: f32) { } verb main() { erg sample: f64 = 1.5; consume(sample: sample); }",
    )
    .expect_err("primitive call types must match");
    assert!(matches!(
        call_error.kind,
        SemanticErrorKind::TypeMismatch { expected, found, .. }
            if expected == "f32" && found == "f64"
    ));

    let assignment_error = analyze_source(
        "verb main() { erg sample: f32 = 1.5; erg other: f64 = 1.5; sample = other; }",
    )
    .expect_err("primitive assignment types must match");
    assert!(matches!(
        assignment_error.kind,
        SemanticErrorKind::BindingTypeMismatch { expected, found, .. }
            if expected == "f32" && found == "f64"
    ));

    let return_error =
        analyze_source("verb produce() -> f32 { erg sample: f64 = 1.5; return sample; }")
            .expect_err("primitive return types must match");
    assert!(matches!(
        return_error.kind,
        SemanticErrorKind::ReturnTypeMismatch { expected, found }
            if expected == "f32" && found == "f64"
    ));
}

#[test]
fn validates_void_as_a_zero_value_return_type() {
    analyze_source("verb main() -> Void { return; }").expect("Void may return without a value");
    let error = analyze_source("verb main() -> Void { return 0; }")
        .expect_err("Void must reject value returns");
    assert!(
        matches!(error.kind, SemanticErrorKind::ReturnTypeMismatch { expected, .. } if expected == "Void")
    );
}

#[test]
fn registry_status_exposes_lifecycle_contract() {
    assert!(RegistryStatus::Active.is_active());
    assert!(!RegistryStatus::Deprecated.is_active());
}

#[test]
fn rejects_unknown_declared_types() {
    let error = analyze_source("verb main(erg buffer: Vector) { }")
        .expect_err("unsupported types must fail before code generation");
    assert!(matches!(error.kind, SemanticErrorKind::UnknownType { name } if name == "Vector"));
}

#[test]
fn rejects_calls_to_unknown_verbs() {
    let error = analyze_source("verb main() { missing_verb(1); }")
        .expect_err("calls must resolve to a declared verb or intrinsic");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::UnknownVerb { name } if name == "missing_verb"
    ));
}

#[test]
fn accepts_registered_collection_types() {
    analyze_source("verb main(erg items: Array[Int, 4], erg index: Map) { }")
        .expect("registered collection types should pass semantic analysis");
}

#[test]
fn validates_intrinsic_argument_types() {
    let handle_error = analyze_source("verb main() { erg value = 1; append(value, 2); }")
        .expect_err("append handle must be Buffer");
    assert!(matches!(
        handle_error.kind,
        SemanticErrorKind::InvalidArgumentRole { callee, parameter }
            if callee == "append" && parameter == "handle"
    ));
}

#[test]
fn infers_buffer_type_from_buffer_literal() {
    analyze_source("verb main() { erg buffer = Buffer[4]; append(buffer, 1); drop(buffer); }")
        .expect("Buffer literal should infer a Buffer owner");
}

#[test]
fn rejects_known_argument_type_mismatches() {
    let error = analyze_source(
        "verb consume(dat buffer: Buffer) { } verb main() { erg value = 1; consume(buffer: value); }",
    )
    .expect_err("known argument type mismatches must fail");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::TypeMismatch { callee, parameter, expected, found }
            if callee == "consume"
                && parameter == "buffer"
                && expected == "Buffer"
                && found == "Int"
    ));
}

#[test]
fn rejects_known_return_type_mismatches() {
    let error = analyze_source("verb main() -> Int { return Buffer[4]; }")
        .expect_err("return type must match the verb declaration");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::ReturnTypeMismatch { expected, found }
            if expected == "Int" && found == "Buffer"
    ));
}

#[test]
fn rejects_typed_initializer_and_assignment_mismatches() {
    let initializer = analyze_source("verb main() { erg buffer: Buffer = 1; }")
        .expect_err("typed initializer must match its binding type");
    assert!(matches!(
        initializer.kind,
        SemanticErrorKind::BindingTypeMismatch { binding, expected, found }
            if binding == "buffer" && expected == "Buffer" && found == "Int"
    ));

    let assignment = analyze_source("verb main() { erg buffer: Buffer = Buffer[1]; buffer = 2; }")
        .expect_err("assignment must match its binding type");
    assert!(matches!(
        assignment.kind,
        SemanticErrorKind::BindingTypeMismatch { binding, expected, found }
            if binding == "buffer" && expected == "Buffer" && found == "Int"
    ));
}

#[test]
fn rejects_assignment_that_changes_an_inferred_primitive_type() {
    let error = analyze_source("verb main() { erg sample = 1.5; sample = 1; }")
        .expect_err("inferred primitive bindings must keep their type");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::BindingTypeMismatch { expected, found, .. }
            if expected == "f64" && found == "Int"
    ));
}

#[test]
fn enforces_return_value_contracts() {
    let missing = analyze_source("verb main() -> Int { erg value = 1; }")
        .expect_err("value-returning verbs must return a value");
    assert!(matches!(missing.kind, SemanticErrorKind::MissingReturnValue));
}

#[test]
fn rejects_return_paths_broken_by_loop_exit() {
    let error = analyze_source("verb main() -> Int { loop { break; return 42; } }")
        .expect_err("a loop break can bypass the return");
    assert!(matches!(error.kind, SemanticErrorKind::MissingReturnValue));
}

#[test]
fn keeps_type_and_function_namespaces_separate() {
    analyze_source(
        "verb Buffer() -> Buffer { return Buffer[1]; } verb main() -> Buffer { return Buffer(); }",
    )
    .expect("a type name and a function name may coexist in separate namespaces");
}

#[test]
fn rejects_duplicate_verbs_in_the_function_namespace() {
    let error = analyze_source("verb main() { } verb main() { }")
        .expect_err("duplicate verb declarations must fail");
    assert!(matches!(error.kind, SemanticErrorKind::DuplicateVerbName { name } if name == "main"));
}

#[test]
fn accepts_buffer_literal_length() {
    analyze_source("verb main() { erg buffer: Buffer = Buffer[4]; drop(buffer); }")
        .expect("Buffer literal lengths should be accepted");
}

#[test]
fn accepts_crc32_over_a_read_only_buffer_range() {
    analyze_source(
        "verb main() -> Int { erg buffer: Buffer = Buffer[4]; return crc32(buffer: abs buffer, start: 0, end: 4); }",
    )
    .expect("crc32 should accept a read-only buffer and integer range");
}

#[test]
fn accepts_crc32_match_validation_over_a_read_only_buffer_range() {
    analyze_source(
        "verb main() -> Int { erg buffer: Buffer = Buffer[4]; return crc32_matches(buffer: abs buffer, start: 0, end: 0, expected: 0); }",
    )
    .expect("crc32_matches should accept a read-only buffer and integer range");
}

#[test]
fn accepts_fixed_frame_validation_over_a_read_only_buffer() {
    analyze_source(
        "verb main() -> Int { erg buffer: Buffer = Buffer[9]; return validate_fixed_frame(buffer: abs buffer, little: 1, version_offset: 0, expected_version: 1, payload_offset: 2, payload_length: 3, checksum_start: 0, checksum_end: 5, checksum_offset: 5); }",
    )
    .expect("validate_fixed_frame should accept a read-only buffer and integer contract values");
}

#[test]
fn rejects_a_non_buffer_crc32_source() {
    let error = analyze_source(
        "verb main() -> Int { erg value: u32 = 1u32; return crc32(buffer: abs value, start: 0, end: 1); }",
    )
    .expect_err("crc32 should reject non-buffer sources");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter }
            if callee == "crc32" && parameter == "buffer"
    ));
}

#[test]
fn rejects_intrinsic_wrong_argument_count() {
    let error =
        analyze_source("verb main() { append(1); }").expect_err("append requires two arguments");
    assert!(
        matches!(error.kind, SemanticErrorKind::WrongArgumentCount { callee } if callee == "append")
    );
}

#[test]
fn rejects_non_integer_buffer_lengths() {
    let error = analyze_source("verb main() { erg buffer = Buffer[\"large\"]; }")
        .expect_err("Buffer length must be an integer");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter } if callee == "Buffer" && parameter == "length")
    );
}

#[test]
fn rejects_borrow_as_append_handle() {
    let error = analyze_source(
        "verb main() { erg buffer: Buffer = Buffer[4]; { abs view = ref buffer; append(view, 1); } }",
    )
    .expect_err("append must require an exclusive owner");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidArgumentRole { callee, parameter } if callee == "append" && parameter == "handle")
    );
}

#[test]
fn rejects_declarations_that_shadow_intrinsics() {
    let error =
        analyze_source("verb allocate() { }").expect_err("intrinsic names must be reserved");
    assert!(
        matches!(error.kind, SemanticErrorKind::ReservedIntrinsicName { name } if name == "allocate")
    );
}

#[test]
fn accepts_typed_float_literals_and_rejects_mismatched_suffixes() {
    analyze_source("verb main() { erg value: f32 = 1.5f32; erg other: f64 = 2.5; }")
        .expect("typed float literals should preserve their declared width");
    let error = analyze_source("verb main() { erg value: f32 = 1.5f64; }")
        .expect_err("a typed f64 literal must not silently narrow to f32");
    assert!(matches!(error.kind, SemanticErrorKind::BindingTypeMismatch { .. }));
}

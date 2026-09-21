use actus::ast::{
    BuiltinType, IntrinsicKind, lookup_builtin_type, lookup_call_intrinsic, lookup_intrinsic,
};
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

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
    assert_eq!(lookup_intrinsic("allocate"), Some(IntrinsicKind::Allocate));
    assert_eq!(IntrinsicKind::Append.spec().parameters, &["handle", "byte"]);
    assert_eq!(lookup_intrinsic("drop"), Some(IntrinsicKind::Drop));
    assert!(lookup_call_intrinsic("drop").is_none());
    assert!(lookup_intrinsic("user_function").is_none());
}

#[test]
fn builtin_type_registry_defines_supported_types() {
    assert_eq!(lookup_builtin_type("Int"), Some(BuiltinType::Int));
    assert_eq!(BuiltinType::Buffer.spec().name, "Buffer");
    assert!(lookup_builtin_type("Array").is_none());
}

#[test]
fn rejects_unknown_declared_types() {
    let error = analyze_source("verb main(erg buffer: Array) { }")
        .expect_err("unsupported types must fail before code generation");
    assert!(matches!(error.kind, SemanticErrorKind::UnknownType { name } if name == "Array"));
}

#[test]
fn validates_intrinsic_argument_types() {
    let length_error =
        analyze_source("verb main() { erg buffer: Buffer = allocate(4); allocate(buffer); }")
            .expect_err("allocate length must be Int");
    assert!(matches!(
        length_error.kind,
        SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter }
            if callee == "allocate" && parameter == "length"
    ));

    let handle_error = analyze_source("verb main() { erg value = 1; append(value, 2); }")
        .expect_err("append handle must be Buffer");
    assert!(matches!(
        handle_error.kind,
        SemanticErrorKind::InvalidArgumentRole { callee, parameter }
            if callee == "append" && parameter == "handle"
    ));
}

#[test]
fn infers_buffer_type_from_allocate_intrinsic() {
    analyze_source("verb main() { erg buffer = allocate(4); append(buffer, 1); drop(buffer); }")
        .expect("allocate should infer a Buffer owner");
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
    let error = analyze_source("verb main() -> Int { return allocate(4); }")
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

    let assignment =
        analyze_source("verb main() { erg buffer: Buffer = allocate(1); buffer = 2; }")
            .expect_err("assignment must match its binding type");
    assert!(matches!(
        assignment.kind,
        SemanticErrorKind::BindingTypeMismatch { binding, expected, found }
            if binding == "buffer" && expected == "Buffer" && found == "Int"
    ));
}

#[test]
fn enforces_return_value_contracts() {
    let missing = analyze_source("verb main() -> Int { erg value = 1; }")
        .expect_err("value-returning verbs must return a value");
    assert!(matches!(missing.kind, SemanticErrorKind::MissingReturnValue));
}

#[test]
fn keeps_type_and_function_namespaces_separate() {
    analyze_source(
        "verb Buffer() -> Buffer { return allocate(1); } verb main() -> Buffer { return Buffer(); }",
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
fn accepts_named_allocate_length() {
    analyze_source("verb main() { erg buffer: Buffer = allocate(length: 4); drop(buffer); }")
        .expect("named intrinsic arguments should be accepted");
}

#[test]
fn rejects_intrinsic_wrong_argument_count() {
    let error =
        analyze_source("verb main() { allocate(); }").expect_err("allocate requires one argument");
    assert!(
        matches!(error.kind, SemanticErrorKind::WrongArgumentCount { callee } if callee == "allocate")
    );

    let error =
        analyze_source("verb main() { append(1); }").expect_err("append requires two arguments");
    assert!(
        matches!(error.kind, SemanticErrorKind::WrongArgumentCount { callee } if callee == "append")
    );
}

#[test]
fn rejects_non_scalar_intrinsic_arguments() {
    let error = analyze_source("verb main() { allocate(length: \"large\"); }")
        .expect_err("allocate length must be scalar");
    assert!(
        matches!(error.kind, SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter } if callee == "allocate" && parameter == "length")
    );
}

#[test]
fn rejects_borrow_as_append_handle() {
    let error = analyze_source(
        "verb main() { erg buffer: Buffer = allocate(4); { abs view = ref buffer; append(view, 1); } }",
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

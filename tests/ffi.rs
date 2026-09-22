use actus::ffi::{
    CAbiError, CAbiLayout, CAbiOwnership, CAbiReturnOwnership, CAbiTarget, CAbiType,
    CallingConvention, c_abi_external_signature, c_abi_signature,
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
        actus::ast::TopLevelDecl::ExternalVerb(_) => panic!("expected regular verb"),
        actus::ast::TopLevelDecl::Struct(_) => panic!("expected regular verb"),
    }
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
fn marks_buffer_returns_as_owned_resources() {
    let verb = parse_verb("verb allocate_buffer() -> Buffer { return allocate(4); }");
    let signature = c_abi_signature(&verb).expect("Buffer return should map");

    assert_eq!(signature.return_ownership, CAbiReturnOwnership::OwnedResource);
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

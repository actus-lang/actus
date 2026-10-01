use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::ast::Program;
use actus::diagnostics::{parse_diagnostic, semantic_diagnostic};
use actus::lexer::scan;
use actus::modules::{ModuleError, ModuleResolutionError, ModuleResolver, analyze_with_imports};
use actus::parser::{ParseError, ParseErrorCode, ParseErrorKind, parse};
use actus::semantic::{SemanticError, SemanticErrorKind, analyze};

fn parse_source(source: &str) -> Result<Program, ParseError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens)
}

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticError> {
    analyze(&parse_source(source).expect("semantic fixture should parse"))
}

fn expect_semantic_error(source: &str) -> SemanticError {
    analyze_source(source).expect_err("negative frontend fixture should fail")
}

#[test]
fn parser_declaration_and_metadata_rules_have_positive_and_negative_fixtures() {
    parse_source("meta target(\"unix\") meta test verb main() -> Int { return 0; }")
        .expect("known declaration and metadata should pass");

    let unknown_keyword = parse_source("verbb main() { return 0; }").expect_err("unknown keyword");
    assert_eq!(unknown_keyword.code, ParseErrorCode::UnknownKeyword);
    assert!(matches!(
        unknown_keyword.kind,
        ParseErrorKind::UnknownKeyword { ref name } if name == "verbb"
    ));
    assert_eq!(parse_diagnostic(&unknown_keyword).code(), "E0009");

    let unknown_metadata =
        parse_source("meta experimental verb main() { return 0; }").expect_err("unknown metadata");
    assert_eq!(unknown_metadata.code, ParseErrorCode::UnknownMetadata);
    assert_eq!(parse_diagnostic(&unknown_metadata).code(), "E0006");

    let unknown_target = parse_source("meta target(\"plan9\") verb main() { return 0; }")
        .expect_err("unknown target");
    assert_eq!(unknown_target.code, ParseErrorCode::UnsupportedTargetPlatform);
    assert_eq!(parse_diagnostic(&unknown_target).code(), "E0007");

    let conflicting_targets =
        parse_source("meta target(\"unix\") meta target(\"windows\") verb main() { return 0; }")
            .expect_err("conflicting targets should be rejected");
    assert_eq!(conflicting_targets.code, ParseErrorCode::ConflictingTargetPlatforms);
    assert_eq!(parse_diagnostic(&conflicting_targets).code(), "E0010");

    let invalid_target =
        parse_source("meta test struct Packet { byte: Int, }").expect_err("metadata on a struct");
    assert_eq!(invalid_target.code, ParseErrorCode::MetadataTargetNotAllowed);
}

#[test]
fn name_resolution_rules_have_positive_and_negative_fixtures() {
    analyze_source(
        "struct Point { x: Int, } verb known(erg point: Point) { } verb main() -> Int { erg point = Point { x: 1, }; known(point: point); return point.x; }",
    )
    .expect("known types, fields, and verbs should pass");

    let unknown_type = expect_semantic_error("verb main(erg item: Missing) { }");
    assert!(
        matches!(unknown_type.kind, SemanticErrorKind::UnknownType { ref name } if name == "Missing")
    );
    assert_eq!(semantic_diagnostic(&unknown_type).code(), "E1023");

    let unknown_field = expect_semantic_error(
        "struct Point { x: Int, } verb main() -> Int { erg point = Point { x: 1, }; return point.missing; }",
    );
    assert!(matches!(
        unknown_field.kind,
        SemanticErrorKind::UnknownStructField { ref field, .. } if field == "missing"
    ));
    assert_eq!(semantic_diagnostic(&unknown_field).code(), "E1031");

    let unknown_verb = expect_semantic_error("verb main() { missing(); }");
    assert!(
        matches!(unknown_verb.kind, SemanticErrorKind::UnknownVerb { ref name } if name == "missing")
    );
    assert_eq!(semantic_diagnostic(&unknown_verb).code(), "E1069");
}

#[test]
fn duplicate_and_call_resolution_rules_have_positive_and_negative_fixtures() {
    analyze_source("struct Left { value: Int, } struct Right { value: Int, }")
        .expect("distinct declarations should pass");

    let duplicate = expect_semantic_error("struct Point { x: Int, } struct Point { y: Int, }");
    assert!(
        matches!(duplicate.kind, SemanticErrorKind::DuplicateStructName { name } if name == "Point")
    );

    analyze_source(
        "verb copy(erg destination: Buffer, erg source: Buffer) { } verb caller() { erg first = Buffer[1]; erg second = Buffer[1]; copy(destination: first, source: second); }",
    )
    .expect("named same-role arguments should disambiguate a call");
    let ambiguous = expect_semantic_error(
        "verb copy(erg destination: Buffer, erg source: Buffer) { } verb caller() { erg first = Buffer[1]; erg second = Buffer[1]; copy(first, second); }",
    );
    assert!(matches!(
        ambiguous.kind,
        SemanticErrorKind::AmbiguousPositionalCall { callee } if callee == "copy"
    ));
}

#[test]
fn return_and_generic_rules_have_positive_and_negative_fixtures() {
    analyze_source("verb answer() -> Int { return 42; }").expect("complete return path");
    let missing_return = expect_semantic_error("verb answer() -> Int { erg result = 42; }");
    assert!(matches!(missing_return.kind, SemanticErrorKind::MissingReturnValue));

    analyze_source("role Reader { verb read(abs self: Int); } struct Box[T: Reader] { item: T, }")
        .expect("declared generic bound should pass");
    let bad_arity =
        expect_semantic_error("struct Box[T] { item: T, } verb main(erg item: Box[Int, Bool]) { }");
    assert!(
        matches!(bad_arity.kind, SemanticErrorKind::GenericArityMismatch { name, .. } if name == "Box")
    );

    let bad_bound = expect_semantic_error(
        "role Reader { verb read(abs self: Int); } struct Other { value: Int, } struct Box[T: Reader] { item: T, } verb main(erg item: Box[Other]) { }",
    );
    assert!(matches!(bad_bound.kind, SemanticErrorKind::GenericConstraintMismatch { .. }));
}

#[test]
fn conditional_guards_require_bool_values() {
    analyze_source("verb choose(erg ready: Bool) { if ready { print(1); } else { print(0); }; }")
        .expect("Bool conditional guard should pass");

    let invalid =
        expect_semantic_error("verb main() -> Int { if 1 { return 1; } else { return 0; } }");
    assert!(matches!(
        invalid.kind,
        SemanticErrorKind::GuardTypeMismatch { ref found } if found == "Int"
    ));
    assert_eq!(semantic_diagnostic(&invalid).code(), "E1063");
}

#[test]
fn malformed_generic_keys_and_primitive_rules_have_fixtures() {
    analyze_source("struct Box[T] { item: T, } verb main(erg item: Box[Int]) { }")
        .expect("well-formed generic key should pass");
    let malformed = SemanticError {
        kind: SemanticErrorKind::MalformedTypeName { name: "Box[".to_owned() },
        span: actus::lexer::SourceSpan::new(0, 4),
    };
    assert_eq!(semantic_diagnostic(&malformed).code(), "E1080");

    analyze_source("verb main(erg bits: u1, erg signed: i8, erg decimal: f32) { }")
        .expect("supported widths and signedness should pass");
    let out_of_range = expect_semantic_error("verb main() { erg bit: u1 = 2; }");
    assert!(matches!(out_of_range.kind, SemanticErrorKind::NumericLiteralOutOfRange { .. }));
}

#[test]
fn module_rules_have_positive_and_negative_fixtures() {
    let fixture = ModuleFixture::new();
    fixture.write("shared/shared.act", "open types;");
    fixture.write("shared/types.act", "open struct Token { byte: Int, }");
    let program = parse_source(
        "import shared; verb main() -> Int { erg token = Token { byte: 7, }; return token.byte; }",
    )
    .expect("valid import should parse");
    analyze_with_imports(&program, &ModuleResolver::new(&fixture.root))
        .expect("valid import and facade export should pass");

    let missing = parse_source("import missing; verb main() { return 0; }")
        .expect("missing import fixture should parse");
    let error = analyze_with_imports(&missing, &ModuleResolver::new(&fixture.root))
        .expect_err("unknown import should fail");
    assert!(matches!(error, ModuleError::Resolution(_)));

    fixture.write("driver/gpio/gpio.act", "");
    fixture.write("driver/gpio.act", "");
    let ambiguous = ModuleResolver::new(&fixture.root)
        .resolve("driver::gpio")
        .expect_err("ambiguous module roots should fail");
    assert!(matches!(ambiguous, ModuleResolutionError::AmbiguousModule { .. }));
}

#[test]
fn non_codegen_declarations_are_explicitly_accepted() {
    analyze_source(
        "struct Empty { } role Marker { } perform Marker for Empty { } unsafe extern \"C\" verb host(abs input: Buffer) -> Int;",
    )
    .expect("explicit non-codegen declarations must remain semantically valid");
}

struct ModuleFixture {
    root: PathBuf,
}

impl ModuleFixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-frontend-completeness-{stamp}"));
        fs::create_dir_all(&root).expect("create module fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, source: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("module fixture parent")).expect("create parent");
        fs::write(path, source).expect("write module fixture");
    }
}

impl Drop for ModuleFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

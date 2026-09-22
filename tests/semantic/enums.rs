use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

#[test]
fn registers_enums_and_validates_unit_and_payload_construction() {
    analyze_source(
        "enum Color { Red, Green, } enum Message { Move(Int, Int), Write { text: String, }, Flag(Bool), } verb main(erg flag: Bool) { erg color: Color = Color.Red; erg move: Message = Message.Move(1, 2); erg write: Message = Message.Write(text: \"ok\"); erg flag_message: Message = Message.Flag(flag); }",
    )
    .expect("valid enum declarations and constructions should pass");
}

#[test]
fn rejects_invalid_enum_variants_and_payload_arguments() {
    let unknown_variant =
        analyze_source("enum Color { Red, } verb main() { erg color: Color = Color.Blue; }")
            .expect_err("unknown enum variants must fail");
    assert!(matches!(
        unknown_variant.kind,
        SemanticErrorKind::UnknownEnumVariant { enum_name, variant }
            if enum_name == "Color" && variant == "Blue"
    ));

    let wrong_count = analyze_source(
        "enum Message { Move(Int, Int), } verb main() { erg message: Message = Message.Move(1); }",
    )
    .expect_err("wrong payload arity must fail");
    assert!(matches!(
        wrong_count.kind,
        SemanticErrorKind::EnumVariantArgumentCount { expected: 2, found: 1, .. }
    ));

    let wrong_type = analyze_source(
        "enum Message { Move(Int), } verb main() { erg message: Message = Message.Move(\"wrong\"); }",
    )
    .expect_err("wrong payload types must fail");
    assert!(matches!(
        wrong_type.kind,
        SemanticErrorKind::EnumVariantArgumentTypeMismatch { expected, found, .. }
            if expected == "Int" && found == "String"
    ));

    let wrong_name = analyze_source(
        "enum Message { Write { text: String, }, } verb main() { erg message: Message = Message.Write(body: \"wrong\"); }",
    )
    .expect_err("unknown named payload fields must fail");
    assert!(matches!(
        wrong_name.kind,
        SemanticErrorKind::EnumVariantArgumentName { name, .. } if name == "body"
    ));
}

#[test]
fn rejects_unknown_and_recursive_enum_payload_types() {
    let unknown_type = analyze_source("enum Message { Invalid(Missing), }")
        .expect_err("unknown enum payload types must fail");
    assert!(matches!(
        unknown_type.kind,
        SemanticErrorKind::UnknownType { name } if name == "Missing"
    ));

    let recursive = analyze_source("enum Node { Next(Node), }")
        .expect_err("direct recursive enum payloads must fail");
    assert!(matches!(
        recursive.kind,
        SemanticErrorKind::RecursiveType { name } if name == "Node"
    ));
}

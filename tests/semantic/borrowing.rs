use actus::ast::Role;
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{AccessState, OwnershipState, SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program)
}

#[test]
fn dat_owner_can_create_a_temporary_abs_view() {
    let model = analyze_source(
        "verb inspect_view(abs view: Buffer) { } verb inspect(dat buffer: Buffer) { { abs view = ref buffer; inspect_view(view); } drop(buffer); }",
    )
    .expect("a dat owner should support temporary abs access");

    let buffer = model.bindings.iter().find(|binding| binding.name == "buffer").unwrap();
    assert_eq!(buffer.role, Role::Dat);
    assert_eq!(buffer.ownership, OwnershipState::Dropped);
    assert_eq!(buffer.access, AccessState::Mutable);
}

#[test]
fn dat_owner_cannot_move_while_temporarily_frozen() {
    let error = analyze_source(
        "verb consume(dat packet: Buffer) { drop(packet); } verb caller(dat buffer: Buffer) { { abs view = ref buffer; consume(packet: buffer); } }",
    )
    .expect_err("a frozen dat owner must not be moved");

    assert!(matches!(error.kind, SemanticErrorKind::MoveFrozen { name, .. } if name == "buffer"));
}

use actus::ast::{ReturnAccess, TopLevelDecl};
use actus::lexer::scan;
use actus::parser::parse;

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

#[test]
fn parses_abs_returns_on_verbs_role_methods_and_external_verbs() {
    let program = parse_source(
        "verb view(abs source: Buffer) -> abs Slice { return source; } role Reader { verb view(abs self: Buffer) -> abs Slice; } unsafe extern \"C\" verb c_view(abs source: Buffer) -> abs Slice;",
    );
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    assert_eq!(verb.return_type.as_ref().unwrap().access, ReturnAccess::Abs);
    let TopLevelDecl::Role(role) = &program.declarations[1] else { panic!("expected role") };
    assert_eq!(role.methods[0].return_type.as_ref().unwrap().access, ReturnAccess::Abs);
    let TopLevelDecl::ExternalVerb(external) = &program.declarations[2] else {
        panic!("expected external verb")
    };
    assert_eq!(external.return_type.as_ref().unwrap().access, ReturnAccess::Abs);
}

#[test]
fn rejects_abs_in_non_return_type_positions() {
    let (tokens, errors) = scan("verb broken(erg input: abs Buffer) -> Int { return 0; }");
    assert!(errors.is_empty());
    assert!(parse(tokens).is_err());

    let (tokens, errors) = scan("verb broken() -> Int abs { return 0; }");
    assert!(errors.is_empty());
    assert!(parse(tokens).is_err());
}

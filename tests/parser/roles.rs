use actus::ast::{Role, TopLevelDecl};
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn parses_role_method_contracts_with_receiver_roles() {
    let (tokens, errors) =
        scan("role Writer { verb write(abs self: File, dat bytes: Buffer) -> Int; }");
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("role declaration should parse");

    let TopLevelDecl::Role(role) = &program.declarations[0] else { panic!("expected role") };
    assert_eq!(role.name, "Writer");
    assert_eq!(role.methods.len(), 1);
    assert_eq!(role.methods[0].params[0].role, Role::Abs);
    assert_eq!(role.methods[0].params[1].role, Role::Dat);
}

#[test]
fn parses_performance_for_a_target_type() {
    let (tokens, errors) = scan(
        "role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return 0; } }",
    );
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("performance declaration should parse");

    let TopLevelDecl::Perform(performance) = &program.declarations[1] else {
        panic!("expected performance")
    };
    assert_eq!(performance.role_name, "Writer");
    assert_eq!(performance.target.name, "File");
    assert_eq!(performance.methods[0].name, "write");
}

use super::DeclarationRegistry;
use crate::ast::{DeclarationIdentity, DeclarationPath};

fn declaration(package: &str, module_path: &str, name: &str) -> DeclarationIdentity {
    DeclarationIdentity::Declaration(DeclarationPath::new(
        super::RegistryToken::new(),
        package,
        module_path,
        name,
    ))
}

#[test]
fn distinct_modules_keep_distinct_declaration_identities() {
    let mut registry = DeclarationRegistry::default();
    registry.register_declaration(
        "app",
        "left",
        &crate::ast::TopLevelDecl::Enum(crate::ast::EnumDef {
            name: "Error".to_owned(),
            doc: None,
            is_open: false,
            metadata: Vec::new(),
            generic_parameters: Vec::new(),
            variants: Vec::new(),
            span: crate::lexer::SourceSpan::new(1, 2),
        }),
    );
    registry.register_declaration(
        "app",
        "right",
        &crate::ast::TopLevelDecl::Enum(crate::ast::EnumDef {
            name: "Error".to_owned(),
            doc: None,
            is_open: false,
            metadata: Vec::new(),
            generic_parameters: Vec::new(),
            variants: Vec::new(),
            span: crate::lexer::SourceSpan::new(3, 4),
        }),
    );
    assert_eq!(registry.resolve("", "left::Error"), Some(declaration("app", "left", "Error")));
    assert_eq!(registry.resolve("", "right::Error"), Some(declaration("app", "right", "Error")));
    assert_ne!(declaration("app", "left", "Error"), declaration("app", "right", "Error"));
}

fn reexport_registry() -> DeclarationRegistry {
    let mut registry = DeclarationRegistry::default();
    registry.register_program(
        "repro_r1",
        "interface::config::contract",
        &crate::ast::Program {
            file_metadata: Vec::new(),
            declarations: vec![crate::ast::TopLevelDecl::Enum(crate::ast::EnumDef {
                name: "ParseError".to_owned(),
                doc: None,
                is_open: true,
                metadata: Vec::new(),
                generic_parameters: Vec::new(),
                variants: Vec::new(),
                span: crate::lexer::SourceSpan::new(1, 2),
            })],
        },
    );
    registry.register_program(
        "repro_r1",
        "app",
        &crate::ast::Program {
            file_metadata: Vec::new(),
            declarations: vec![
                crate::ast::TopLevelDecl::Import(crate::ast::ImportDecl {
                    path: "interface::config::contract".to_owned(),
                    alias: None,
                    is_open: false,
                    selected: Vec::new(),
                    grouped: false,
                    span: crate::lexer::SourceSpan::new(3, 4),
                }),
                crate::ast::TopLevelDecl::Import(crate::ast::ImportDecl {
                    path: "interface::config::contract".to_owned(),
                    alias: Some("config".to_owned()),
                    is_open: false,
                    selected: Vec::new(),
                    grouped: false,
                    span: crate::lexer::SourceSpan::new(4, 5),
                }),
                crate::ast::TopLevelDecl::Import(crate::ast::ImportDecl {
                    path: "interface::config::contract".to_owned(),
                    alias: None,
                    is_open: true,
                    selected: Vec::new(),
                    grouped: false,
                    span: crate::lexer::SourceSpan::new(5, 6),
                }),
            ],
        },
    );
    registry
}

#[test]
fn one_declaration_has_one_identity_across_reexport_spellings() {
    let registry = reexport_registry();
    let expected = declaration("repro_r1", "interface::config::contract", "ParseError");
    for spelling in ["interface::config::contract::ParseError", "config::ParseError", "ParseError"]
    {
        assert_eq!(registry.resolve("app", spelling), Some(expected.clone()));
    }
    assert!(matches!(
        registry.resolve("other", "ParseError"),
        Some(DeclarationIdentity::Unknown { .. })
    ));
}

#[test]
fn identity_resolution_is_independent_of_registration_order() {
    let mut first = DeclarationRegistry::default();
    let mut second = DeclarationRegistry::default();
    let declaration = crate::ast::TopLevelDecl::Enum(crate::ast::EnumDef {
        name: "ParseError".to_owned(),
        doc: None,
        is_open: false,
        metadata: Vec::new(),
        generic_parameters: Vec::new(),
        variants: Vec::new(),
        span: crate::lexer::SourceSpan::new(1, 2),
    });
    first.register_declaration("app", "interface::contract", &declaration);
    second.register_declaration("app", "interface::contract", &declaration);
    assert_eq!(
        first.resolve("interface::contract", "ParseError"),
        second.resolve("interface::contract", "ParseError")
    );
}

#[test]
fn reserved_root_module_path_is_distinct_from_user_root_module() {
    let mut registry = DeclarationRegistry::default();
    let value = crate::ast::TopLevelDecl::Enum(crate::ast::EnumDef {
        name: "Error".to_owned(),
        doc: None,
        is_open: false,
        metadata: Vec::new(),
        generic_parameters: Vec::new(),
        variants: Vec::new(),
        span: crate::lexer::SourceSpan::new(1, 2),
    });
    registry.register_declaration("app", super::ROOT_MODULE_PATH, &value);
    registry.register_declaration("app", "root", &value);
    assert_eq!(
        registry.identity_for(super::ROOT_MODULE_PATH, "Error"),
        Some(declaration("app", super::ROOT_MODULE_PATH, "Error"))
    );
    assert_ne!(
        declaration("app", super::ROOT_MODULE_PATH, "Error"),
        declaration("app", "root", "Error")
    );
}

#[test]
fn const_generic_literals_have_value_identity() {
    let registry = DeclarationRegistry::default();
    let mut source = crate::ast::TypeName::unresolved(
        "Array",
        vec![
            crate::ast::TypeName::unresolved(
                "u8",
                Vec::new(),
                None,
                crate::lexer::SourceSpan::new(1, 2),
            ),
            crate::ast::TypeName::unresolved(
                "064",
                Vec::new(),
                None,
                crate::lexer::SourceSpan::new(3, 6),
            ),
        ],
        None,
        crate::lexer::SourceSpan::new(1, 6),
    );
    registry.resolve_type_name("<root>", &mut source);
    let sixty_four = crate::ast::TypeName::constant("64", "64", source.span);
    assert_eq!(source.arguments[1].resolved, sixty_four.resolved);

    let mut other = crate::ast::TypeName::unresolved(
        "Array",
        vec![
            crate::ast::TypeName::unresolved(
                "u8",
                Vec::new(),
                None,
                crate::lexer::SourceSpan::new(1, 2),
            ),
            crate::ast::TypeName::unresolved(
                "65",
                Vec::new(),
                None,
                crate::lexer::SourceSpan::new(3, 5),
            ),
        ],
        None,
        crate::lexer::SourceSpan::new(1, 5),
    );
    registry.resolve_type_name("<root>", &mut other);
    assert_ne!(source.arguments[1].resolved, other.arguments[1].resolved);
}

#[test]
fn user_result_identity_is_distinct_from_builtin_result() {
    let mut registry = DeclarationRegistry::default();
    registry.register_declaration(
        "app",
        super::ROOT_MODULE_PATH,
        &crate::ast::TopLevelDecl::Enum(crate::ast::EnumDef {
            name: "Result".to_owned(),
            doc: None,
            is_open: false,
            metadata: Vec::new(),
            generic_parameters: Vec::new(),
            variants: Vec::new(),
            span: crate::lexer::SourceSpan::new(1, 2),
        }),
    );
    let mut user = crate::ast::TypeName::unresolved(
        "Result",
        Vec::new(),
        None,
        crate::lexer::SourceSpan::new(1, 7),
    );
    registry.resolve_type_name(super::ROOT_MODULE_PATH, &mut user);
    let builtin = crate::ast::TypeName::builtin(
        "Result",
        Vec::new(),
        None,
        crate::lexer::SourceSpan::new(1, 7),
    );
    assert!(matches!(user.resolved_identity(), Some(DeclarationIdentity::Declaration(_))));
    assert_ne!(user, builtin);
}

#[test]
fn unknown_type_resolves_to_diagnostic_identity() {
    let registry = DeclarationRegistry::default();
    let mut missing = crate::ast::TypeName::unresolved(
        "Missing",
        Vec::new(),
        None,
        crate::lexer::SourceSpan::new(1, 8),
    );
    registry.resolve_type_name(super::ROOT_MODULE_PATH, &mut missing);
    assert_eq!(
        missing.resolved_identity(),
        Some(&DeclarationIdentity::Unknown { spelling: "Missing".to_owned() })
    );
}

#[test]
fn annotates_type_constructor_callee_arguments() {
    let span = crate::lexer::SourceSpan::new(1, 10);
    let call = crate::ast::Expr::Call {
        callee: "Array[u32,2]".to_owned(),
        resolved_type: None,
        generic_arguments: Vec::new(),
        arguments: Vec::new(),
        span,
    };
    let mut program = crate::ast::Program {
        file_metadata: Vec::new(),
        declarations: vec![crate::ast::TopLevelDecl::Verb(crate::ast::VerbDecl {
            is_open: false,
            doc: None,
            contract: None,
            metadata: Vec::new(),
            name: "main".to_owned(),
            generic_parameters: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: crate::ast::Block {
                statements: vec![crate::ast::Stmt::Expression { expression: call, span }],
                span,
            },
            span,
        })],
    };
    DeclarationRegistry::resolve_standalone(&mut program);
    let crate::ast::TopLevelDecl::Verb(verb) = &program.declarations[0] else {
        panic!("expected verb fixture")
    };
    let crate::ast::Stmt::Expression {
        expression: crate::ast::Expr::Call { resolved_type, .. },
        ..
    } = &verb.body.statements[0]
    else {
        panic!("expected call fixture")
    };
    let type_name = resolved_type.as_ref().expect("constructor callee should be annotated");
    assert!(matches!(
        type_name.resolved_identity(),
        Some(DeclarationIdentity::Builtin { name }) if name == "Array"
    ));
    assert!(matches!(
        type_name.arguments[0].resolved_identity(),
        Some(DeclarationIdentity::Builtin { name }) if name == "u32"
    ));
    assert_eq!(
        type_name.arguments[1].resolved_identity(),
        Some(&DeclarationIdentity::Const { value: "2".to_owned() })
    );
}

fn standalone_program(source: &str) -> crate::ast::Program {
    let (tokens, errors) = crate::lexer::scan(source);
    assert!(errors.is_empty());
    let mut program = crate::parser::parse(tokens).expect("generic scope source parses");
    DeclarationRegistry::resolve_standalone(&mut program);
    program
}

fn first_parameter_identity(declaration: &crate::ast::TopLevelDecl) -> &DeclarationIdentity {
    let crate::ast::TopLevelDecl::Verb(verb) = declaration else { panic!("expected a verb") };
    verb.params[0].ty.resolved_identity().expect("parameter type is resolved")
}

#[test]
fn generic_parameters_belong_to_their_own_declaration() {
    let program = standalone_program(
        "verb first[T](abs value: T) { return; } verb second[T](abs value: T) { return; }",
    );
    let first = first_parameter_identity(&program.declarations[0]);
    let second = first_parameter_identity(&program.declarations[1]);
    let DeclarationIdentity::GenericParameter { owner: first_owner, name } = first else {
        panic!("`T` of `first` is a generic parameter, got {first:?}")
    };
    let DeclarationIdentity::GenericParameter { owner: second_owner, .. } = second else {
        panic!("`T` of `second` is a generic parameter, got {second:?}")
    };
    assert_eq!(name, "T");
    assert!(first_owner.ends_with("::first"), "{first_owner}");
    assert!(second_owner.ends_with("::second"), "{second_owner}");
    assert_ne!(first, second);
}

#[test]
fn generic_parameter_does_not_leak_into_other_declarations() {
    let program = standalone_program(
        "verb generic[T](abs value: T) { return; } verb plain(abs value: T) { return; }",
    );
    assert!(matches!(
        first_parameter_identity(&program.declarations[0]),
        DeclarationIdentity::GenericParameter { .. }
    ));
    assert_eq!(
        first_parameter_identity(&program.declarations[1]),
        &DeclarationIdentity::Unknown { spelling: "T".to_owned() }
    );
}

#[test]
fn body_owner_type_is_annotated_as_builtin() {
    let program = standalone_program("verb main() -> Int { erg value: Int = 1; return value; }");
    let crate::ast::TopLevelDecl::Verb(verb) = &program.declarations[0] else {
        panic!("expected a verb")
    };
    let crate::ast::Stmt::OwnerDecl { resolved_type: Some(owner_type), .. } =
        &verb.body.statements[0]
    else {
        panic!("owner declaration has an annotated type")
    };
    assert_eq!(
        owner_type.resolved_identity(),
        Some(&DeclarationIdentity::Builtin { name: "Int".to_owned() })
    );
}

#[test]
fn spelling_resolves_in_the_scope_of_the_resolving_module() {
    let mut registry = DeclarationRegistry::default();
    let error = |name: &str| {
        crate::ast::TopLevelDecl::Enum(crate::ast::EnumDef {
            name: name.to_owned(),
            doc: None,
            is_open: true,
            metadata: Vec::new(),
            generic_parameters: Vec::new(),
            variants: Vec::new(),
            span: crate::lexer::SourceSpan::new(1, 2),
        })
    };
    registry.register_declaration("std", "std::io", &error("IoError"));
    registry.register_declaration("app", super::ROOT_MODULE_PATH, &error("IoError"));
    assert_eq!(
        registry.resolve("std::io", "IoError"),
        Some(declaration("std", "std::io", "IoError"))
    );
    assert_eq!(
        registry.resolve(super::ROOT_MODULE_PATH, "IoError"),
        Some(declaration("app", super::ROOT_MODULE_PATH, "IoError"))
    );
}

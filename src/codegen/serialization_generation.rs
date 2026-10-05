use crate::ast::{
    Argument, ArgumentRoleResolution, BinaryOp, Block, EnumDef, EnumPayload, EnumVariant, Expr,
    GenericParam, IfBranch, PackStorage, Param, Program, ReturnAccess, ReturnType, Role,
    SerializeDecl, Stmt, TopLevelDecl, TypeName, VerbDecl,
};
use crate::lexer::SourceSpan;

const GENERATED_DOC: &str = "Compiler-generated fixed-frame serialization wrapper.";

pub(super) fn append_builtin_serialization_error(program: &mut Program) {
    if program.declarations.iter().any(|declaration| {
        matches!(declaration, TopLevelDecl::Enum(definition) if definition.name == "SerializationError")
    }) {
        return;
    }
    let span = SourceSpan::new(0, 0);
    let variants = [
        "BufferTooSmall",
        "InvalidVersion",
        "InvalidChecksum",
        "InvalidLayout",
        "UnsupportedVersion",
    ]
    .into_iter()
    .map(|name| EnumVariant { doc: None, name: name.to_owned(), payload: EnumPayload::Unit, span })
    .collect();
    program.declarations.push(TopLevelDecl::Enum(EnumDef {
        is_open: false,
        doc: Some(GENERATED_DOC.to_owned()),
        name: "SerializationError".to_owned(),
        generic_parameters: Vec::<GenericParam>::new(),
        variants,
        span,
    }));
}

pub(super) fn append_generated_serialization_encoders(program: &mut Program) {
    let contracts = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Serialize(contract) => Some(contract.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for contract in contracts {
        let Some(pack) = program.declarations.iter().find_map(|declaration| match declaration {
            TopLevelDecl::Pack(pack) if pack.name == contract.source_type.name => Some(pack),
            _ => None,
        }) else {
            continue;
        };
        let Some(PackStorage::ByteArray { capacity, .. }) = Some(&pack.storage) else {
            continue;
        };
        program.declarations.push(TopLevelDecl::Verb(generated_encoder(
            &contract,
            &pack.name,
            *capacity,
            contract.span,
        )));
    }
}

fn generated_encoder(
    contract: &SerializeDecl,
    source_type: &str,
    capacity: u64,
    span: SourceSpan,
) -> VerbDecl {
    let pack_type = type_name(source_type, span);
    let output_type = type_name("Buffer", span);
    let return_type = result_type("u32", span);
    let mut statements = Vec::new();
    for index in 0..capacity {
        statements.push(append_guard(index, span));
    }
    statements.push(Stmt::Return { value: Some(result_ok("u32", capacity, span)), span });
    VerbDecl {
        is_open: false,
        doc: Some(GENERATED_DOC.to_owned()),
        metadata: Vec::new(),
        name: format!("{}_encode", contract.name.to_ascii_lowercase()),
        generic_parameters: Vec::new(),
        params: vec![
            Param {
                role: Role::Abs,
                name: "value".to_owned(),
                dispatch: crate::ast::DispatchMode::Static,
                ty: pack_type,
                span,
            },
            Param {
                role: Role::Ins,
                name: "output".to_owned(),
                dispatch: crate::ast::DispatchMode::Static,
                ty: output_type,
                span,
            },
        ],
        return_type: Some(ReturnType { access: ReturnAccess::Owned, ty: return_type, span }),
        body: Block { statements, span },
        span,
    }
}

fn append_guard(index: u64, span: SourceSpan) -> Stmt {
    let call = Expr::Call {
        callee: "append".to_owned(),
        arguments: vec![
            inferred_argument(identifier("output", span)),
            inferred_argument(storage_byte(index, span)),
        ],
        span,
    };
    Stmt::If {
        condition: Expr::Binary {
            left: Box::new(Expr::Cast {
                expression: Box::new(call),
                target: TypeName {
                    name: "u8".to_owned(),
                    arguments: Vec::new(),
                    reference_role: None,
                    span,
                },
                span,
            }),
            operator: BinaryOp::Equals,
            right: Box::new(Expr::Integer {
                value: "1".to_owned(),
                suffix: Some("u8".to_owned()),
                span,
            }),
            span,
        },
        then_branch: Block { statements: Vec::new(), span },
        else_branch: Some(IfBranch::Block(Block {
            statements: vec![Stmt::Return {
                value: Some(result_err("u32", "BufferTooSmall", span)),
                span,
            }],
            span,
        })),
        span,
    }
}

fn storage_byte(index: u64, span: SourceSpan) -> Expr {
    Expr::Index {
        target: Box::new(Expr::FieldAccess {
            object: Box::new(identifier("value", span)),
            field: "storage".to_owned(),
            span,
        }),
        index: Box::new(Expr::Integer { value: index.to_string(), suffix: None, span }),
        span,
    }
}

fn result_ok(value_type: &str, value: u64, span: SourceSpan) -> Expr {
    result_constructor(
        value_type,
        "Ok",
        Expr::Integer { value: value.to_string(), suffix: Some(value_type.to_owned()), span },
        span,
    )
}

fn result_err(value_type: &str, variant: &str, span: SourceSpan) -> Expr {
    result_constructor(
        value_type,
        "Err",
        Expr::FieldAccess {
            object: Box::new(identifier("SerializationError", span)),
            field: variant.to_owned(),
            span,
        },
        span,
    )
}

fn result_constructor(value_type: &str, method: &str, value: Expr, span: SourceSpan) -> Expr {
    Expr::MethodCall {
        receiver: Box::new(identifier(&format!("Result[{value_type},SerializationError]"), span)),
        method: method.to_owned(),
        arguments: vec![argument(None, Role::Erg, value)],
        span,
    }
}

fn argument(name: Option<&str>, role: Role, expression: Expr) -> Argument {
    let role_span = Some(expression_span(&expression));
    Argument {
        name: name.map(str::to_owned),
        role: Some(role),
        role_span,
        role_resolution: ArgumentRoleResolution::Explicit,
        expression,
    }
}

fn inferred_argument(expression: Expr) -> Argument {
    Argument {
        name: None,
        role: None,
        role_span: None,
        role_resolution: ArgumentRoleResolution::Unspecified,
        expression,
    }
}

fn identifier(name: &str, span: SourceSpan) -> Expr {
    Expr::Identifier { name: name.to_owned(), span }
}

fn type_name(name: &str, span: SourceSpan) -> TypeName {
    TypeName { name: name.to_owned(), arguments: Vec::new(), reference_role: None, span }
}

fn result_type(value: &str, span: SourceSpan) -> TypeName {
    TypeName {
        name: "Result".to_owned(),
        arguments: vec![type_name(value, span), type_name("SerializationError", span)],
        reference_role: None,
        span,
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. } => *span,
        _ => SourceSpan::new(0, 0),
    }
}

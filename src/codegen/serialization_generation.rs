use crate::ast::{
    Argument, ArgumentRoleResolution, BinaryOp, Block, EnumDef, EnumPayload, EnumVariant, Expr,
    GenericParam, IfBranch, PackStorage, Param, Place, Program, ReturnAccess, ReturnType, Role,
    SerializeDecl, Stmt, StructFieldInit, TopLevelDecl, TypeName, VerbDecl,
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

pub(super) fn append_generated_serialization_decoders(program: &mut Program) {
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
        program.declarations.push(TopLevelDecl::Verb(generated_decoder(
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
    statements.push(Stmt::Return {
        value: Some(result_ok("u32", integer(capacity, Some("u32"), span), span)),
        span,
    });
    VerbDecl {
        is_open: false,
        doc: Some(GENERATED_DOC.to_owned()),
        contract: None,
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

fn generated_decoder(
    contract: &SerializeDecl,
    source_type: &str,
    capacity: u64,
    span: SourceSpan,
) -> VerbDecl {
    let mut statements = decoder_prefix(source_type, capacity, span);
    statements.push(version_guard(contract, span));
    statements.push(checksum_guard(contract, span));
    append_decoder_assignments(&mut statements, capacity, span);
    statements.push(Stmt::Return {
        value: Some(short_result_constructor("Ok", identifier("decoded", span), span)),
        span,
    });
    VerbDecl {
        is_open: false,
        doc: Some(GENERATED_DOC.to_owned()),
        contract: None,
        metadata: Vec::new(),
        name: format!("{}_decode", contract.name.to_ascii_lowercase()),
        generic_parameters: Vec::new(),
        params: vec![
            Param {
                role: Role::Abs,
                name: "input".to_owned(),
                dispatch: crate::ast::DispatchMode::Static,
                ty: type_name("Buffer", span),
                span,
            },
            Param {
                role: Role::Abs,
                name: "expected_version".to_owned(),
                dispatch: crate::ast::DispatchMode::Static,
                ty: type_name("u16", span),
                span,
            },
        ],
        return_type: Some(ReturnType {
            access: ReturnAccess::Owned,
            ty: result_type(source_type, span),
            span,
        }),
        body: Block { statements, span },
        span,
    }
}

fn decoder_prefix(source_type: &str, capacity: u64, span: SourceSpan) -> Vec<Stmt> {
    let mut statements = vec![Stmt::OwnerDecl {
        role: Role::Erg,
        name: "input_length".to_owned(),
        ty: Some("Int".to_owned()),
        initializer: Expr::Call {
            callee: "buffer_length".to_owned(),
            arguments: vec![inferred_argument(identifier("input", span))],
            span,
        },
        span,
    }];
    statements.push(Stmt::If {
        condition: Expr::Binary {
            left: Box::new(identifier("input_length", span)),
            operator: BinaryOp::LessThan,
            right: Box::new(integer(capacity, None, span)),
            span,
        },
        then_branch: Block {
            statements: vec![Stmt::Return {
                value: Some(result_err(source_type, "InvalidLayout", synthetic_span(span, 2))),
                span,
            }],
            span,
        },
        else_branch: None,
        span,
    });
    statements.push(Stmt::OwnerDecl {
        role: Role::Erg,
        name: "decoded".to_owned(),
        ty: None,
        initializer: pack_zero_value(source_type, capacity, span),
        span,
    });
    statements
}

fn version_guard(contract: &SerializeDecl, span: SourceSpan) -> Stmt {
    let offset = contract
        .sections
        .iter()
        .find_map(|section| match section {
            crate::ast::SerializeSection::Version { offset, .. } => Some(*offset),
            _ => None,
        })
        .unwrap_or(0);
    Stmt::If {
        condition: Expr::Binary {
            left: Box::new(version_value(contract, offset, span)),
            operator: BinaryOp::NotEquals,
            right: Box::new(identifier("expected_version", span)),
            span,
        },
        then_branch: Block {
            statements: vec![Stmt::Return {
                value: Some(result_err(
                    contract.source_type.name.as_str(),
                    "InvalidVersion",
                    synthetic_span(span, 3),
                )),
                span,
            }],
            span,
        },
        else_branch: None,
        span,
    }
}

fn checksum_guard(contract: &SerializeDecl, span: SourceSpan) -> Stmt {
    let call = Expr::Call {
        callee: format!("{}_validate", contract.name.to_ascii_lowercase()),
        arguments: vec![
            argument(Some("frame"), Role::Abs, identifier("input", span)),
            argument(Some("expected_version"), Role::Abs, identifier("expected_version", span)),
        ],
        span,
    };
    Stmt::If {
        condition: Expr::Binary {
            left: Box::new(call),
            operator: BinaryOp::Equals,
            right: Box::new(integer(0, Some("i64"), span)),
            span,
        },
        then_branch: Block {
            statements: vec![Stmt::Return {
                value: Some(result_err(
                    contract.source_type.name.as_str(),
                    "InvalidChecksum",
                    synthetic_span(span, 4),
                )),
                span,
            }],
            span,
        },
        else_branch: None,
        span,
    }
}

fn version_value(contract: &SerializeDecl, offset: u16, span: SourceSpan) -> Expr {
    let first = cast_u16(input_byte(u64::from(offset), span), span);
    let second = cast_u16(input_byte(u64::from(offset.saturating_add(1)), span), span);
    let (high, low) = match contract.endianness {
        crate::ast::LayoutEndianness::Little => (second, first),
        crate::ast::LayoutEndianness::Big => (first, second),
    };
    Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(high),
            operator: BinaryOp::ShiftLeft,
            right: Box::new(integer(8, Some("u16"), span)),
            span,
        }),
        operator: BinaryOp::BitwiseOr,
        right: Box::new(low),
        span,
    }
}

fn cast_u16(expression: Expr, span: SourceSpan) -> Expr {
    Expr::Cast { expression: Box::new(expression), target: type_name("u16", span), span }
}

fn append_decoder_assignments(statements: &mut Vec<Stmt>, capacity: u64, span: SourceSpan) {
    for index in 0..capacity {
        statements.push(Stmt::Assignment {
            target: Place::Index {
                target: Box::new(Place::Field {
                    object: Box::new(Place::Binding { name: "decoded".to_owned(), span }),
                    field: "storage".to_owned(),
                    span,
                }),
                index: integer(index, None, span),
                span,
            },
            value: input_byte(index, span),
            span,
        });
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

fn input_byte(index: u64, span: SourceSpan) -> Expr {
    Expr::Index {
        target: Box::new(identifier("input", span)),
        index: Box::new(integer(index, None, span)),
        span,
    }
}

fn pack_zero_value(source_type: &str, capacity: u64, span: SourceSpan) -> Expr {
    Expr::StructLit {
        name: source_type.to_owned(),
        type_arguments: Vec::new(),
        fields: vec![StructFieldInit {
            name: "storage".to_owned(),
            value: Expr::Call {
                callee: format!("Array[u8,{capacity}]"),
                arguments: Vec::new(),
                span,
            },
            span,
        }],
        span,
    }
}

fn result_ok(value_type: &str, value: impl Into<Expr>, span: SourceSpan) -> Expr {
    result_constructor(value_type, "Ok", value.into(), span)
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

fn short_result_constructor(method: &str, value: Expr, span: SourceSpan) -> Expr {
    Expr::Call {
        callee: method.to_owned(),
        arguments: vec![inferred_argument(value)],
        span: synthetic_span(span, 1),
    }
}

fn result_constructor(value_type: &str, method: &str, value: Expr, span: SourceSpan) -> Expr {
    Expr::MethodCall {
        receiver: Box::new(identifier(&format!("Result[{value_type},SerializationError]"), span)),
        method: method.to_owned(),
        arguments: vec![argument(None, Role::Erg, value)],
        span,
    }
}

fn integer(value: u64, suffix: Option<&str>, span: SourceSpan) -> Expr {
    Expr::Integer { value: value.to_string(), suffix: suffix.map(str::to_owned), span }
}

fn synthetic_span(span: SourceSpan, offset: usize) -> SourceSpan {
    SourceSpan::new(span.end.saturating_add(offset), span.end.saturating_add(offset + 1))
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

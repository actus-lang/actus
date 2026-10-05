use crate::ast::{
    Argument, ArgumentRoleResolution, BinaryOp, Block, Expr, PackStorage, Param, Program,
    ReturnAccess, ReturnType, Role, SerializeDecl, SerializeSection, Stmt, TopLevelDecl, TypeName,
    VerbDecl,
};
use crate::lexer::SourceSpan;

const GENERATED_DOC: &str = "Compiler-generated fixed-frame serialization wrapper.";

pub(super) fn append_generated_serialization_migrations(program: &mut Program) {
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
        program.declarations.push(TopLevelDecl::Verb(generated_migration(
            &contract,
            *capacity,
            contract.span,
        )));
    }
}

fn generated_migration(contract: &SerializeDecl, capacity: u64, span: SourceSpan) -> VerbDecl {
    let mut statements = vec![buffer_length_guard(capacity, span), empty_output_guard(span)];
    statements.push(version_guard(contract, span));
    statements.push(checksum_guard(contract, span));
    append_copy_guards(&mut statements, capacity, span);
    append_version_writes(&mut statements, contract, span);
    append_checksum_write(&mut statements, contract, span);
    statements.push(Stmt::Return { value: Some(result_ok(capacity, span)), span });
    VerbDecl {
        is_open: false,
        doc: Some(GENERATED_DOC.to_owned()),
        metadata: Vec::new(),
        name: format!("{}_migrate", contract.name.to_ascii_lowercase()),
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
                name: "from_version".to_owned(),
                dispatch: crate::ast::DispatchMode::Static,
                ty: type_name("u16", span),
                span,
            },
            Param {
                role: Role::Abs,
                name: "to_version".to_owned(),
                dispatch: crate::ast::DispatchMode::Static,
                ty: type_name("u16", span),
                span,
            },
            Param {
                role: Role::Ins,
                name: "output".to_owned(),
                dispatch: crate::ast::DispatchMode::Static,
                ty: type_name("Buffer", span),
                span,
            },
        ],
        return_type: Some(ReturnType { access: ReturnAccess::Owned, ty: result_type(span), span }),
        body: Block { statements, span },
        span,
    }
}

fn buffer_length_guard(capacity: u64, span: SourceSpan) -> Stmt {
    Stmt::If {
        condition: Expr::Binary {
            left: Box::new(Expr::Call {
                callee: "buffer_length".to_owned(),
                arguments: vec![inferred(identifier("input", span))],
                span,
            }),
            operator: BinaryOp::LessThan,
            right: Box::new(integer(capacity, None, span)),
            span,
        },
        then_branch: return_block("InvalidLayout", 2, span),
        else_branch: None,
        span,
    }
}

fn empty_output_guard(span: SourceSpan) -> Stmt {
    Stmt::If {
        condition: Expr::Binary {
            left: Box::new(Expr::Call {
                callee: "buffer_length".to_owned(),
                arguments: vec![inferred(identifier("output", span))],
                span,
            }),
            operator: BinaryOp::NotEquals,
            right: Box::new(integer(0, None, span)),
            span,
        },
        then_branch: return_block("InvalidLayout", 7, span),
        else_branch: None,
        span,
    }
}

fn version_guard(contract: &SerializeDecl, span: SourceSpan) -> Stmt {
    let offset = version_offset(contract);
    Stmt::If {
        condition: Expr::Binary {
            left: Box::new(version_value(contract, "input", offset, span)),
            operator: BinaryOp::NotEquals,
            right: Box::new(identifier("from_version", span)),
            span,
        },
        then_branch: return_block("InvalidVersion", 3, span),
        else_branch: None,
        span,
    }
}

fn checksum_guard(contract: &SerializeDecl, span: SourceSpan) -> Stmt {
    let call = Expr::Call {
        callee: format!("{}_validate", contract.name.to_ascii_lowercase()),
        arguments: vec![
            named("frame", Role::Abs, identifier("input", span)),
            named("expected_version", Role::Abs, identifier("from_version", span)),
        ],
        span,
    };
    Stmt::If {
        condition: Expr::Binary {
            left: Box::new(call),
            operator: BinaryOp::Equals,
            right: Box::new(integer(0, None, span)),
            span,
        },
        then_branch: return_block("InvalidChecksum", 4, span),
        else_branch: None,
        span,
    }
}

fn append_copy_guards(statements: &mut Vec<Stmt>, capacity: u64, span: SourceSpan) {
    for index in 0..capacity {
        let append = Expr::Call {
            callee: "append".to_owned(),
            arguments: vec![
                inferred(identifier("output", span)),
                inferred(input_byte(index, span)),
            ],
            span,
        };
        statements.push(Stmt::If {
            condition: Expr::Binary {
                left: Box::new(Expr::Cast {
                    expression: Box::new(append),
                    target: type_name("u8", span),
                    span,
                }),
                operator: BinaryOp::Equals,
                right: Box::new(integer(1, Some("u8"), span)),
                span,
            },
            then_branch: Block { statements: Vec::new(), span },
            else_branch: Some(crate::ast::IfBranch::Block(return_block_block(
                "BufferTooSmall",
                5,
                span,
            ))),
            span,
        });
    }
}

fn append_version_writes(statements: &mut Vec<Stmt>, contract: &SerializeDecl, span: SourceSpan) {
    let offset = version_offset(contract);
    let low = cast_u8(identifier("to_version", span), span);
    let high = cast_u8(
        Expr::Binary {
            left: Box::new(identifier("to_version", span)),
            operator: BinaryOp::ShiftRight,
            right: Box::new(integer(8, Some("u16"), span)),
            span,
        },
        span,
    );
    let values = match contract.endianness {
        crate::ast::LayoutEndianness::Little => [low, high],
        crate::ast::LayoutEndianness::Big => [high, low],
    };
    for (index, value) in values.into_iter().enumerate() {
        statements.push(assign_output(offset.saturating_add(index as u16), value, span));
    }
}

fn append_checksum_write(statements: &mut Vec<Stmt>, contract: &SerializeDecl, span: SourceSpan) {
    let Some((start, end, offset)) = checksum_section(contract) else { return };
    statements.push(Stmt::OwnerDecl {
        role: Role::Erg,
        name: "checksum".to_owned(),
        ty: Some("i64".to_owned()),
        initializer: cast_i64(
            Expr::Call {
                callee: "crc32".to_owned(),
                arguments: vec![
                    named("buffer", Role::Abs, identifier("output", span)),
                    named("start", Role::Abs, integer(i64::from(start) as u64, None, span)),
                    named("end", Role::Abs, integer(i64::from(end) as u64, None, span)),
                ],
                span,
            },
            span,
        ),
        span,
    });
    for index in 0..4u16 {
        let shifted = masked_checksum_byte(index, span);
        statements.push(assign_output(offset.saturating_add(index), cast_u8(shifted, span), span));
    }
}

fn masked_checksum_byte(index: u16, span: SourceSpan) -> Expr {
    Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(identifier("checksum", span)),
            operator: BinaryOp::ShiftRight,
            right: Box::new(integer(u64::from(index) * 8, Some("u8"), span)),
            span,
        }),
        operator: BinaryOp::BitwiseAnd,
        right: Box::new(integer(255, Some("i64"), span)),
        span,
    }
}

fn assign_output(index: u16, value: Expr, span: SourceSpan) -> Stmt {
    Stmt::Assignment {
        target: crate::ast::Place::Index {
            target: Box::new(crate::ast::Place::Binding { name: "output".to_owned(), span }),
            index: integer(u64::from(index), None, span),
            span,
        },
        value,
        span,
    }
}

fn version_value(contract: &SerializeDecl, binding: &str, offset: u16, span: SourceSpan) -> Expr {
    let first = cast_u16(input_byte_for(binding, offset, span), span);
    let second = cast_u16(input_byte_for(binding, offset.saturating_add(1), span), span);
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

fn return_block(variant: &str, offset: usize, span: SourceSpan) -> Block {
    Block {
        statements: vec![Stmt::Return { value: Some(result_err(variant, offset, span)), span }],
        span,
    }
}

fn return_block_block(variant: &str, offset: usize, span: SourceSpan) -> Block {
    return_block(variant, offset, span)
}

fn result_err(variant: &str, offset: usize, span: SourceSpan) -> Expr {
    Expr::MethodCall {
        receiver: Box::new(identifier("Result[u32,SerializationError]", span)),
        method: "Err".to_owned(),
        arguments: vec![argument(
            None,
            Role::Erg,
            Expr::FieldAccess {
                object: Box::new(identifier("SerializationError", span)),
                field: variant.to_owned(),
                span: synthetic_span(span, offset),
            },
        )],
        span: synthetic_span(span, offset),
    }
}

fn result_ok(value: u64, span: SourceSpan) -> Expr {
    let result_span = synthetic_span(span, 6);
    Expr::MethodCall {
        receiver: Box::new(identifier("Result[u32,SerializationError]", result_span)),
        method: "Ok".to_owned(),
        arguments: vec![argument(None, Role::Erg, integer(value, Some("u32"), result_span))],
        span: result_span,
    }
}

fn checksum_section(contract: &SerializeDecl) -> Option<(u16, u16, u16)> {
    contract.sections.iter().find_map(|section| match section {
        SerializeSection::Checksum { start, end, offset, .. } => Some((*start, *end, *offset)),
        _ => None,
    })
}

fn version_offset(contract: &SerializeDecl) -> u16 {
    contract
        .sections
        .iter()
        .find_map(|section| match section {
            SerializeSection::Version { offset, .. } => Some(*offset),
            _ => None,
        })
        .unwrap_or(0)
}

fn input_byte(index: u64, span: SourceSpan) -> Expr {
    input_byte_for("input", u16::try_from(index).unwrap_or(u16::MAX), span)
}

fn input_byte_for(binding: &str, index: u16, span: SourceSpan) -> Expr {
    Expr::Index {
        target: Box::new(identifier(binding, span)),
        index: Box::new(integer(u64::from(index), None, span)),
        span,
    }
}

fn cast_u16(expression: Expr, span: SourceSpan) -> Expr {
    Expr::Cast { expression: Box::new(expression), target: type_name("u16", span), span }
}

fn cast_u8(expression: Expr, span: SourceSpan) -> Expr {
    Expr::Cast { expression: Box::new(expression), target: type_name("u8", span), span }
}

fn cast_i64(expression: Expr, span: SourceSpan) -> Expr {
    Expr::Cast { expression: Box::new(expression), target: type_name("i64", span), span }
}

fn result_type(span: SourceSpan) -> TypeName {
    TypeName {
        name: "Result".to_owned(),
        arguments: vec![type_name("u32", span), type_name("SerializationError", span)],
        reference_role: None,
        span,
    }
}

fn type_name(name: &str, span: SourceSpan) -> TypeName {
    TypeName { name: name.to_owned(), arguments: Vec::new(), reference_role: None, span }
}

fn integer(value: u64, suffix: Option<&str>, span: SourceSpan) -> Expr {
    Expr::Integer { value: value.to_string(), suffix: suffix.map(str::to_owned), span }
}

fn identifier(name: &str, span: SourceSpan) -> Expr {
    Expr::Identifier { name: name.to_owned(), span }
}

fn named(name: &str, role: Role, expression: Expr) -> Argument {
    argument(Some(name), role, expression)
}

fn inferred(expression: Expr) -> Argument {
    Argument {
        name: None,
        role: None,
        role_span: None,
        role_resolution: ArgumentRoleResolution::Unspecified,
        expression,
    }
}

fn argument(name: Option<&str>, role: Role, expression: Expr) -> Argument {
    Argument {
        name: name.map(str::to_owned),
        role: Some(role),
        role_span: Some(expression_span(&expression)),
        role_resolution: ArgumentRoleResolution::Explicit,
        expression,
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::Call { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Binary { span, .. } => *span,
        _ => SourceSpan::new(0, 0),
    }
}

fn synthetic_span(span: SourceSpan, offset: usize) -> SourceSpan {
    SourceSpan::new(span.end.saturating_add(offset), span.end.saturating_add(offset + 1))
}

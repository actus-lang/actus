use std::collections::HashMap;

use crate::ast::{
    Argument, ArgumentRoleResolution, Block, CaseBody, DispatchMode, Expr, Param, Program,
    ReturnAccess, ReturnType, Role, SerializeDecl, SerializeSection, Stmt, TopLevelDecl, TypeName,
    VerbDecl,
};
use crate::lexer::SourceSpan;

/// Rewrites contextual `Ok` and `Err` calls into the existing enum-constructor
/// representation before native lowering. The rewrite is compile-time only;
/// the generated code uses the ordinary `Result` enum layout.
pub(crate) fn normalize_program(program: &Program) -> Program {
    let mut normalized = program.clone();
    super::serialization_generation::append_builtin_serialization_error(&mut normalized);
    append_generated_serialization_validators(&mut normalized);
    super::serialization_generation::append_generated_serialization_encoders(&mut normalized);
    let signatures = collect_signatures(&normalized);
    for declaration in &mut normalized.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => normalize_verb(verb, &signatures),
            TopLevelDecl::Perform(perform) => {
                for method in &mut perform.methods {
                    normalize_verb(method, &signatures);
                }
            }
            _ => {}
        }
    }
    super::constants::inline_constants(&mut normalized);
    normalized
}
fn append_generated_serialization_validators(program: &mut Program) {
    let contracts = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Serialize(contract) => Some(contract.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for contract in contracts {
        program.declarations.push(TopLevelDecl::Verb(generated_validator(&contract)));
    }
}
fn generated_validator(contract: &SerializeDecl) -> VerbDecl {
    let span = contract.span;
    let buffer =
        TypeName { name: "Buffer".to_owned(), arguments: Vec::new(), reference_role: None, span };
    let version =
        TypeName { name: "u16".to_owned(), arguments: Vec::new(), reference_role: None, span };
    let call = Expr::Call {
        callee: "validate_fixed_frame".to_owned(),
        arguments: validator_arguments(contract, span),
        span,
    };
    VerbDecl {
        is_open: false,
        doc: Some("Compiler-generated fixed-frame serialization wrapper.".to_owned()),
        metadata: Vec::new(),
        name: format!("{}_validate", contract.name.to_ascii_lowercase()),
        generic_parameters: Vec::new(),
        params: vec![
            Param {
                role: Role::Abs,
                name: "frame".to_owned(),
                dispatch: DispatchMode::Static,
                ty: buffer,
                span,
            },
            Param {
                role: Role::Erg,
                name: "expected_version".to_owned(),
                dispatch: DispatchMode::Static,
                ty: version,
                span,
            },
        ],
        return_type: Some(ReturnType {
            access: ReturnAccess::Owned,
            ty: TypeName {
                name: "Int".to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span,
            },
            span,
        }),
        body: Block { statements: vec![Stmt::Return { value: Some(call), span }], span },
        span,
    }
}
fn validator_arguments(contract: &SerializeDecl, span: SourceSpan) -> Vec<Argument> {
    let fields =
        contract.sections.iter().fold((None, None, None), |state, section| match section {
            SerializeSection::Version { offset, .. } => (Some(*offset), state.1, state.2),
            SerializeSection::Payload { offset, length, .. } => {
                (state.0, Some((*offset, *length)), state.2)
            }
            SerializeSection::Checksum { start, end, offset, .. } => {
                (state.0, state.1, Some((*start, *end, *offset)))
            }
        });
    let (
        Some(version_offset),
        Some((payload_offset, payload_length)),
        Some((checksum_start, checksum_end, checksum_offset)),
    ) = fields
    else {
        return Vec::new();
    };
    vec![
        named_argument("buffer", Role::Abs, identifier("frame", span)),
        integer_argument(
            "little",
            u16::from(contract.endianness == crate::ast::LayoutEndianness::Little),
            span,
        ),
        integer_argument("version_offset", version_offset, span),
        named_argument("expected_version", Role::Abs, identifier("expected_version", span)),
        integer_argument("payload_offset", payload_offset, span),
        integer_argument("payload_length", payload_length, span),
        integer_argument("checksum_start", checksum_start, span),
        integer_argument("checksum_end", checksum_end, span),
        integer_argument("checksum_offset", checksum_offset, span),
    ]
}
fn named_argument(name: &str, role: Role, expression: Expr) -> Argument {
    Argument {
        name: Some(name.to_owned()),
        role: Some(role),
        role_span: Some(expression_span(&expression)),
        role_resolution: ArgumentRoleResolution::Explicit,
        expression,
    }
}
fn integer_argument(name: &str, value: u16, span: SourceSpan) -> Argument {
    named_argument(name, Role::Abs, Expr::Integer { value: value.to_string(), suffix: None, span })
}
fn identifier(name: &str, span: SourceSpan) -> Expr {
    Expr::Identifier { name: name.to_owned(), span }
}
fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => *span,
    }
}

fn normalize_verb(
    verb: &mut crate::ast::VerbDecl,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
) {
    let return_type = verb.return_type.as_ref().map(|return_type| return_type.ty.clone());
    let mut locals = verb
        .params
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
        .collect();
    normalize_block(&mut verb.body, return_type.as_ref(), signatures, &mut locals);
}

fn collect_signatures(program: &Program) -> HashMap<String, Vec<(String, TypeName)>> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => Some((
                verb.name.clone(),
                verb.params
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
                    .collect(),
            )),
            TopLevelDecl::ExternalVerb(verb) => Some((
                verb.name.clone(),
                verb.params
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

fn normalize_block(
    block: &mut Block,
    expected: Option<&TypeName>,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    for statement in &mut block.statements {
        normalize_statement(statement, expected, signatures, locals);
    }
}

fn normalize_statement(
    statement: &mut Stmt,
    return_type: Option<&TypeName>,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    match statement {
        Stmt::OwnerDecl { name, ty, initializer, .. } => {
            normalize_owner_declaration(name, ty.as_deref(), initializer, signatures, locals)
        }
        Stmt::Assignment { target, value, .. } => {
            let mut target_expression = target.to_expr();
            normalize_expression(&mut target_expression, None, signatures, locals);
            normalize_expression(value, None, signatures, locals);
        }
        Stmt::CompoundAssignment { target, value, .. } => {
            let mut target_expression = target.to_expr();
            normalize_expression(&mut target_expression, None, signatures, locals);
            normalize_expression(value, None, signatures, locals);
        }
        Stmt::Expression { expression, .. } => {
            normalize_expression(expression, return_type, signatures, locals);
        }
        Stmt::If { condition, then_branch, else_branch, .. } => {
            normalize_expression(condition, None, signatures, locals);
            normalize_block(then_branch, return_type, signatures, locals);
            if let Some(crate::ast::IfBranch::Block(block)) = else_branch {
                normalize_block(block, return_type, signatures, locals);
            }
        }
        Stmt::Return { value: Some(expression), .. } => {
            normalize_expression(expression, return_type, signatures, locals);
        }
        Stmt::Block(nested) | Stmt::Loop(nested) => {
            normalize_block(nested, return_type, signatures, locals);
        }
        Stmt::ForRange { start, end, body, .. } => {
            normalize_expression(start, None, signatures, locals);
            normalize_expression(end, None, signatures, locals);
            normalize_block(body, return_type, signatures, locals);
        }
        Stmt::ForArray { collection, body, .. } => {
            normalize_expression(collection, None, signatures, locals);
            normalize_block(body, return_type, signatures, locals);
        }
        Stmt::Return { value: None, .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Drop { .. } => {}
    }
}

fn normalize_owner_declaration(
    name: &str,
    declared_type: Option<&str>,
    initializer: &mut Expr,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    let expected =
        declared_type.and_then(|name| parse_type_name(name, initializer_span(initializer)));
    normalize_expression(initializer, expected.as_ref(), signatures, locals);
    if let Some(expected) = expected {
        locals.insert(name.to_owned(), expected);
    }
}

fn normalize_expression(
    expression: &mut Expr,
    expected: Option<&TypeName>,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    match expression {
        Expr::Call { .. } => normalize_call(expression, expected, signatures, locals),
        Expr::MethodCall { receiver, arguments, .. } => {
            normalize_method_call(receiver, arguments, signatures, locals)
        }
        Expr::Grouping { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Try { expression, .. } => {
            normalize_expression(expression, expected, signatures, locals);
        }
        Expr::Binary { left, right, .. } => {
            normalize_binary(left, right, signatures, locals);
        }
        Expr::Case { .. } => normalize_case(expression, expected, signatures, locals),
        Expr::If { .. } => {}
        Expr::StructLit { fields, .. } => normalize_struct_fields(fields, signatures, locals),
        Expr::FieldAccess { object, .. } => {
            normalize_expression(object, None, signatures, locals);
        }
        Expr::Index { target, index, .. } => {
            normalize_expression(target, None, signatures, locals);
            normalize_expression(index, None, signatures, locals);
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. }
        | Expr::BufferLiteral { .. } => {}
    }
}

fn normalize_method_call(
    receiver: &mut Expr,
    arguments: &mut [Argument],
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    normalize_expression(receiver, None, signatures, locals);
    for argument in arguments {
        normalize_expression(&mut argument.expression, None, signatures, locals);
    }
}

fn normalize_binary(
    left: &mut Expr,
    right: &mut Expr,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    normalize_expression(left, None, signatures, locals);
    normalize_expression(right, None, signatures, locals);
}

fn normalize_struct_fields(
    fields: &mut [crate::ast::StructFieldInit],
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    for field in fields {
        normalize_expression(&mut field.value, None, signatures, locals);
    }
}

fn normalize_call(
    expression: &mut Expr,
    expected: Option<&TypeName>,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    let Expr::Call { callee, arguments, .. } = expression else { return };
    let Some(result_type) =
        expected.filter(|type_name| type_name.name == "Result" && type_name.arguments.len() == 2)
    else {
        normalize_arguments(callee, arguments, signatures, locals);
        return;
    };
    if !matches!(callee.as_str(), "Ok" | "Err") || arguments.len() != 1 {
        normalize_arguments(callee, arguments, signatures, locals);
        return;
    }
    rewrite_result_call(expression, result_type, signatures, locals);
}

fn rewrite_result_call(
    expression: &mut Expr,
    result_type: &TypeName,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    let Expr::Call { callee, arguments, span } = expression else { return };
    let payload_index = usize::from(callee == "Err");
    normalize_expression(
        &mut arguments[0].expression,
        result_type.arguments.get(payload_index),
        signatures,
        locals,
    );
    let receiver = Expr::Identifier { name: canonical_type_name(result_type), span: *span };
    let method = callee.clone();
    let rewritten_arguments = std::mem::take(arguments);
    *expression = Expr::MethodCall {
        receiver: Box::new(receiver),
        method,
        arguments: rewritten_arguments,
        span: *span,
    };
}

fn normalize_case(
    expression: &mut Expr,
    expected: Option<&TypeName>,
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    let Expr::Case { subject, branches, .. } = expression else { return };
    normalize_expression(subject, None, signatures, locals);
    for branch in branches {
        match &mut branch.body {
            CaseBody::Expression(expression) => {
                normalize_expression(expression, expected, signatures, locals)
            }
            CaseBody::Block(block) => normalize_block(block, expected, signatures, locals),
        }
        if let Some(guard) = &mut branch.guard {
            normalize_expression(guard, None, signatures, locals);
        }
    }
}

fn normalize_arguments(
    callee: &str,
    arguments: &mut [Argument],
    signatures: &HashMap<String, Vec<(String, TypeName)>>,
    locals: &mut HashMap<String, TypeName>,
) {
    let Some(parameters) = signatures.get(callee) else {
        for argument in arguments {
            normalize_expression(&mut argument.expression, None, signatures, locals);
        }
        return;
    };
    for (index, argument) in arguments.iter_mut().enumerate() {
        let parameter = argument
            .name
            .as_ref()
            .and_then(|name| parameters.iter().find(|(parameter, _)| parameter == name))
            .or_else(|| parameters.get(index));
        normalize_expression(
            &mut argument.expression,
            parameter.map(|(_, type_name)| type_name),
            signatures,
            locals,
        );
    }
}

fn parse_type_name(input: &str, span: crate::lexer::SourceSpan) -> Option<TypeName> {
    let Some(open) = input.find('[') else {
        return Some(TypeName {
            name: input.to_owned(),
            arguments: Vec::new(),
            reference_role: None,
            span,
        });
    };
    if !input.ends_with(']') {
        return None;
    }
    let inner = &input[open + 1..input.len() - 1];
    let arguments = inner
        .split(',')
        .map(|argument| parse_type_name(argument.trim(), span))
        .collect::<Option<Vec<_>>>()?;
    Some(TypeName { name: input[..open].to_owned(), arguments, reference_role: None, span })
}

fn canonical_type_name(type_name: &TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

fn initializer_span(expression: &Expr) -> crate::lexer::SourceSpan {
    match expression {
        Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Try { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. }
        | Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Cast { span, .. } => *span,
    }
}

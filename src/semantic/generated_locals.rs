use std::collections::HashMap;

use crate::ast::{
    Argument, Block, Expr, IfBranch, Param, Place, Role, Stmt, TopLevelDecl, TypeName,
};
use crate::lexer::SourceSpan;

#[derive(Clone)]
struct ParameterSpec {
    name: String,
    role: Role,
    ty: String,
}

type SignatureMap = HashMap<String, Vec<ParameterSpec>>;

pub(crate) fn normalize_program(program: &crate::ast::Program) -> crate::ast::Program {
    let signatures = signatures(program);
    let mut normalized = program.clone();
    let mut counter = 0usize;
    for declaration in &mut normalized.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => {
                normalize_block(&mut verb.body, &signatures, &mut counter);
            }
            TopLevelDecl::Perform(perform) => {
                for method in &mut perform.methods {
                    normalize_block(&mut method.body, &signatures, &mut counter);
                }
            }
            _ => {}
        }
    }
    normalized
}

fn signatures(program: &crate::ast::Program) -> SignatureMap {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => Some((verb.name.clone(), parameter_specs(&verb.params))),
            _ => None,
        })
        .collect()
}

fn parameter_specs(params: &[Param]) -> Vec<ParameterSpec> {
    params
        .iter()
        .map(|param| ParameterSpec {
            name: param.name.clone(),
            role: param.role.clone(),
            ty: type_name(&param.ty),
        })
        .collect()
}

fn normalize_block(block: &mut Block, signatures: &SignatureMap, counter: &mut usize) {
    let statements = std::mem::take(&mut block.statements);
    let mut normalized = Vec::with_capacity(statements.len());
    for mut statement in statements {
        let mut generated = Vec::new();
        normalize_statement(&mut statement, signatures, counter, &mut generated);
        normalized.extend(generated);
        normalized.push(statement);
    }
    block.statements = normalized;
}

fn normalize_statement(
    statement: &mut Stmt,
    signatures: &SignatureMap,
    counter: &mut usize,
    generated: &mut Vec<Stmt>,
) {
    match statement {
        Stmt::OwnerDecl { initializer, .. } | Stmt::Return { value: Some(initializer), .. } => {
            let (prefix, expression) = normalize_expression(initializer, signatures, counter);
            generated.extend(prefix);
            *initializer = expression;
        }
        Stmt::Expression { expression, .. } => {
            let (prefix, normalized) = normalize_expression(expression, signatures, counter);
            generated.extend(prefix);
            *expression = normalized;
        }
        Stmt::Assignment { target, value, .. } | Stmt::CompoundAssignment { target, value, .. } => {
            let (prefix, expression) = normalize_expression(value, signatures, counter);
            generated.extend(prefix);
            *value = expression;
            normalize_place(target, signatures, counter, generated);
        }
        Stmt::If { condition, then_branch, else_branch, .. } => {
            let (prefix, expression) = normalize_expression(condition, signatures, counter);
            generated.extend(prefix);
            *condition = expression;
            normalize_block(then_branch, signatures, counter);
            if let Some(IfBranch::Block(block)) = else_branch {
                normalize_block(block, signatures, counter);
            }
        }
        Stmt::Loop(block) | Stmt::Block(block) => normalize_block(block, signatures, counter),
        Stmt::ForRange { start, end, body, .. } => {
            let (prefix, expression) = normalize_expression(start, signatures, counter);
            generated.extend(prefix);
            *start = expression;
            let (prefix, expression) = normalize_expression(end, signatures, counter);
            generated.extend(prefix);
            *end = expression;
            normalize_block(body, signatures, counter);
        }
        Stmt::ForArray { collection, body, .. } => {
            let (prefix, expression) = normalize_expression(collection, signatures, counter);
            generated.extend(prefix);
            *collection = expression;
            normalize_block(body, signatures, counter);
        }
        Stmt::Return { value: None, .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Drop { .. } => {}
    }
}

fn normalize_place(
    place: &mut Place,
    signatures: &SignatureMap,
    counter: &mut usize,
    generated: &mut Vec<Stmt>,
) {
    if let Place::Index { target, index, .. } = place {
        normalize_place(target, signatures, counter, generated);
        let (prefix, expression) = normalize_nested_expression(index, signatures, counter);
        generated.extend(prefix);
        *index = expression;
    } else if let Place::Field { object, .. } = place {
        normalize_place(object, signatures, counter, generated);
    }
}

fn normalize_expression(
    expression: &mut Expr,
    signatures: &SignatureMap,
    counter: &mut usize,
) -> (Vec<Stmt>, Expr) {
    let Expr::Call { callee, arguments, span } = expression else {
        return (Vec::new(), expression.clone());
    };
    let mut prefix = Vec::new();
    materialize_arguments(callee, arguments, *span, signatures, counter, &mut prefix);
    (prefix, expression.clone())
}

fn normalize_nested_expression(
    expression: &mut Expr,
    _signatures: &SignatureMap,
    _counter: &mut usize,
) -> (Vec<Stmt>, Expr) {
    (Vec::new(), expression.clone())
}

fn materialize_arguments(
    callee: &str,
    arguments: &mut [Argument],
    call_span: SourceSpan,
    signatures: &SignatureMap,
    counter: &mut usize,
    prefix: &mut Vec<Stmt>,
) {
    let Some(parameters) = signatures.get(callee) else { return };
    for (index, argument) in arguments.iter_mut().enumerate() {
        let Some(parameter) = argument_parameter(argument, index, parameters) else { continue };
        if argument.role != Some(Role::Abs)
            || parameter.role != Role::Abs
            || !is_scalar_type(&parameter.ty)
            || !is_pure_scalar(&argument.expression)
            || matches!(argument.expression, Expr::Identifier { .. })
        {
            continue;
        }
        let name = format!("__actus_generated_{counter}");
        *counter += 1;
        let initializer = argument.expression.clone();
        let span = expression_span(&initializer).unwrap_or(call_span);
        prefix.push(Stmt::OwnerDecl {
            role: Role::Erg,
            name: name.clone(),
            ty: Some(parameter.ty.clone()),
            initializer,
            span,
        });
        argument.expression = Expr::Identifier { name, span };
    }
}

fn argument_parameter<'a>(
    argument: &Argument,
    index: usize,
    parameters: &'a [ParameterSpec],
) -> Option<&'a ParameterSpec> {
    argument
        .name
        .as_ref()
        .and_then(|name| parameters.iter().find(|parameter| parameter.name == *name))
        .or_else(|| parameters.get(index))
}

fn is_scalar_type(name: &str) -> bool {
    matches!(
        name,
        "Int"
            | "Bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "f32"
            | "f64"
    )
}

fn is_pure_scalar(expression: &Expr) -> bool {
    match expression {
        Expr::Integer { .. } | Expr::FloatLiteral { .. } | Expr::BoolLiteral { .. } => true,
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. } => is_pure_scalar(expression),
        Expr::Binary { left, right, .. } => is_pure_scalar(left) && is_pure_scalar(right),
        Expr::Index { target, index, .. } => is_pure_scalar(target) && is_pure_scalar(index),
        Expr::Identifier { .. } | Expr::FieldAccess { .. } => true,
        _ => false,
    }
}

fn expression_span(expression: &Expr) -> Option<SourceSpan> {
    Some(match expression {
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
    })
}

fn type_name(value: &TypeName) -> String {
    if value.arguments.is_empty() {
        value.name.clone()
    } else {
        format!(
            "{}[{}]",
            value.name,
            value
                .arguments
                .iter()
                .map(|argument| type_name(argument))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_source(source: &str) -> crate::ast::Program {
        let (tokens, errors) = crate::lexer::scan(source);
        assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
        crate::parser::parse(tokens).expect("source should parse")
    }

    #[test]
    fn materializes_pure_scalar_abs_arguments_in_source_order() {
        let program = parse_source(
            "verb read(abs value: u32) { return; } verb main() { erg index: u32 = 1u32; read(value: abs (index + 1u32)); }",
        );
        let normalized = normalize_program(&program);
        let TopLevelDecl::Verb(main) = &normalized.declarations[1] else {
            panic!("expected main verb")
        };
        assert_eq!(main.body.statements.len(), 3);
        let Stmt::OwnerDecl { name, role, ty, .. } = &main.body.statements[1] else {
            panic!("expected generated owner")
        };
        assert_eq!(name, "__actus_generated_0");
        assert_eq!(*role, Role::Erg);
        assert_eq!(ty.as_deref(), Some("u32"));
        let Stmt::Expression { expression: Expr::Call { arguments, .. }, .. } =
            &main.body.statements[2]
        else {
            panic!("expected normalized call")
        };
        assert!(matches!(
            arguments[0].expression,
            Expr::Identifier { ref name, .. } if name == "__actus_generated_0"
        ));
        crate::semantic::analyze(&program).expect("generated scalar call should remain valid");
    }

    #[test]
    fn leaves_calls_outside_the_pure_scalar_boundary_unchanged() {
        let program = parse_source(
            "verb read(abs value: u32) { return; } verb make() -> u32 { return 1u32; } verb main() { read(value: abs make()); }",
        );
        let normalized = normalize_program(&program);
        let TopLevelDecl::Verb(main) = &normalized.declarations[2] else {
            panic!("expected main verb")
        };
        assert_eq!(main.body.statements.len(), 1);
        let Stmt::Expression { expression: Expr::Call { arguments, .. }, .. } =
            &main.body.statements[0]
        else {
            panic!("expected original call")
        };
        assert!(
            matches!(arguments[0].expression, Expr::Call { ref callee, .. } if callee == "make")
        );
    }
}

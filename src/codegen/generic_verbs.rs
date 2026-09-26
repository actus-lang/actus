use crate::ast::{Block, Expr, Param, Program, ReturnType, Stmt, TopLevelDecl, TypeName, VerbDecl};
use crate::semantic::{GenericInstance, TypeSubstitution};

use super::generic_definitions::canonical_type_name;
use super::native::NativeEmitError;

/// Materializes generic verbs for concrete type applications discovered by the
/// semantic pass. The resulting declarations have no generic parameters and
/// can therefore enter the ordinary Cranelift declaration pipeline.
pub(super) fn specialize_program(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Program, NativeEmitError> {
    let mut declarations = Vec::with_capacity(program.declarations.len());
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) if !verb.generic_parameters.is_empty() => {
                if let Some(specialized) = specialize_verb(verb, instances)? {
                    declarations.push(TopLevelDecl::Verb(specialized));
                } else {
                    declarations.push(declaration.clone());
                }
            }
            other => declarations.push(other.clone()),
        }
    }
    Ok(Program { declarations })
}

fn specialize_verb(
    verb: &VerbDecl,
    instances: &[GenericInstance],
) -> Result<Option<VerbDecl>, NativeEmitError> {
    let Some(instance) = instances.iter().find(|instance| instance_matches_verb(instance, verb))
    else {
        return Ok(None);
    };
    let substitution = TypeSubstitution::for_type(
        &verb.name,
        &verb.generic_parameters,
        &instance.arguments,
        verb.span,
    )
    .map_err(|error| NativeEmitError(format!("generic verb substitution failed: {error:?}")))?;
    Ok(Some(VerbDecl {
        is_open: verb.is_open,
        metadata: verb.metadata.clone(),
        name: verb.name.clone(),
        generic_parameters: Vec::new(),
        params: verb.params.iter().map(|param| specialize_param(param, &substitution)).collect(),
        return_type: verb
            .return_type
            .as_ref()
            .map(|return_type| specialize_return_type(return_type, &substitution)),
        body: specialize_block(&verb.body, &substitution),
        span: verb.span,
    }))
}

fn instance_matches_verb(instance: &GenericInstance, verb: &VerbDecl) -> bool {
    instance.arguments.len() == verb.generic_parameters.len()
        && (verb.params.iter().any(|param| type_contains_application(&param.ty, &instance.name))
            || verb.return_type.as_ref().is_some_and(|return_type| {
                type_contains_application(&return_type.ty, &instance.name)
            }))
}

fn type_contains_application(type_name: &TypeName, name: &str) -> bool {
    type_name.name == name
        || type_name.arguments.iter().any(|argument| type_contains_application(argument, name))
}

fn specialize_param(param: &Param, substitution: &TypeSubstitution) -> Param {
    Param { ty: substitution.apply(&param.ty), ..param.clone() }
}

fn specialize_return_type(return_type: &ReturnType, substitution: &TypeSubstitution) -> ReturnType {
    ReturnType { ty: substitution.apply(&return_type.ty), ..return_type.clone() }
}

fn specialize_block(block: &Block, substitution: &TypeSubstitution) -> Block {
    Block {
        statements: block
            .statements
            .iter()
            .map(|statement| specialize_statement(statement, substitution))
            .collect(),
        span: block.span,
    }
}

fn specialize_statement(statement: &Stmt, substitution: &TypeSubstitution) -> Stmt {
    match statement {
        Stmt::OwnerDecl { role, name, ty, span, initializer } => Stmt::OwnerDecl {
            role: role.clone(),
            name: name.clone(),
            ty: ty.clone(),
            initializer: specialize_expression(initializer, substitution),
            span: *span,
        },
        Stmt::Assignment { name, value, span } => Stmt::Assignment {
            name: name.clone(),
            value: specialize_expression(value, substitution),
            span: *span,
        },
        Stmt::FieldAssignment { object, field, value, span } => Stmt::FieldAssignment {
            object: specialize_expression(object, substitution),
            field: field.clone(),
            value: specialize_expression(value, substitution),
            span: *span,
        },
        Stmt::Expression { expression, span } => Stmt::Expression {
            expression: specialize_expression(expression, substitution),
            span: *span,
        },
        Stmt::Return { value: Some(value), span } => {
            Stmt::Return { value: Some(specialize_expression(value, substitution)), span: *span }
        }
        Stmt::Loop(block) => Stmt::Loop(specialize_block(block, substitution)),
        Stmt::Block(block) => Stmt::Block(specialize_block(block, substitution)),
        _ => statement.clone(),
    }
}

fn specialize_expression(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    match expression {
        Expr::Grouping { expression: child, span } => Expr::Grouping {
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Borrow { expression: child, span } => Expr::Borrow {
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Try { expression: child, span } => Expr::Try {
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Unary { operator, expression: child, span } => Expr::Unary {
            operator: *operator,
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Binary { .. }
        | Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. }
        | Expr::Case { .. } => specialize_complex_expression(expression, substitution),
        _ => expression.clone(),
    }
}

fn specialize_complex_expression(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    match expression {
        Expr::Binary { left, operator, right, span } => Expr::Binary {
            left: Box::new(specialize_expression(left, substitution)),
            operator: *operator,
            right: Box::new(specialize_expression(right, substitution)),
            span: *span,
        },
        Expr::Call { callee, arguments, span } => Expr::Call {
            callee: callee.clone(),
            arguments: specialize_arguments(arguments, substitution),
            span: *span,
        },
        Expr::MethodCall { receiver, method, arguments, span } => Expr::MethodCall {
            receiver: Box::new(specialize_expression(receiver, substitution)),
            method: method.clone(),
            arguments: specialize_arguments(arguments, substitution),
            span: *span,
        },
        Expr::StructLit { name, type_arguments, fields, span } => Expr::StructLit {
            name: name.clone(),
            type_arguments: type_arguments.iter().map(|ty| substitution.apply(ty)).collect(),
            fields: fields
                .iter()
                .map(|field| crate::ast::StructFieldInit {
                    value: specialize_expression(&field.value, substitution),
                    ..field.clone()
                })
                .collect(),
            span: *span,
        },
        Expr::FieldAccess { object, field, span } => Expr::FieldAccess {
            object: Box::new(specialize_expression(object, substitution)),
            field: field.clone(),
            span: *span,
        },
        Expr::Case { mode, subject, branches, span } => Expr::Case {
            mode: *mode,
            subject: Box::new(specialize_expression(subject, substitution)),
            branches: branches
                .iter()
                .map(|branch| crate::ast::CaseBranch {
                    guard: branch
                        .guard
                        .as_ref()
                        .map(|guard| Box::new(specialize_expression(guard, substitution))),
                    body: specialize_case_body(&branch.body, substitution),
                    ..branch.clone()
                })
                .collect(),
            span: *span,
        },
        _ => expression.clone(),
    }
}

fn specialize_case_body(
    body: &crate::ast::CaseBody,
    substitution: &TypeSubstitution,
) -> crate::ast::CaseBody {
    match body {
        crate::ast::CaseBody::Expression(value) => {
            crate::ast::CaseBody::Expression(Box::new(specialize_expression(value, substitution)))
        }
        crate::ast::CaseBody::Block(block) => {
            crate::ast::CaseBody::Block(specialize_block(block, substitution))
        }
    }
}

fn specialize_arguments(
    arguments: &[crate::ast::Argument],
    substitution: &TypeSubstitution,
) -> Vec<crate::ast::Argument> {
    arguments
        .iter()
        .map(|argument| crate::ast::Argument {
            expression: specialize_expression(&argument.expression, substitution),
            ..argument.clone()
        })
        .collect()
}

#[allow(dead_code)]
fn _specialization_key(instance: &GenericInstance) -> String {
    canonical_type_name(&TypeName {
        name: instance.name.clone(),
        arguments: instance.arguments.clone(),
        span: crate::lexer::SourceSpan::new(0, 0),
    })
}

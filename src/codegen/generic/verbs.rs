use crate::ast::{
    Block, Expr, ExternalVerbDecl, Param, Program, ReturnType, Stmt, TopLevelDecl, TypeName,
    VerbDecl,
};
use crate::semantic::{GenericInstance, TypeSubstitution};

use super::super::native::NativeEmitError;
use super::definitions::canonical_type_name;

/// Materializes generic verbs for concrete type applications discovered by the
/// semantic pass. The resulting declarations have no generic parameters and
/// can therefore enter the ordinary Cranelift declaration pipeline.
pub(in crate::codegen) fn specialize_program(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Program, NativeEmitError> {
    let mut declarations = Vec::with_capacity(program.declarations.len());
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) if !verb.generic_parameters.is_empty() => {
                if let Some(specialized) = specialize_verb(verb, instances)? {
                    declarations.push(TopLevelDecl::Verb(specialized));
                }
            }
            TopLevelDecl::ExternalVerb(verb) if !verb.generic_parameters.is_empty() => {
                if let Some(specialized) = specialize_external_verb(verb, instances)? {
                    declarations.push(TopLevelDecl::ExternalVerb(specialized));
                }
            }
            other => declarations.push(other.clone()),
        }
    }
    Ok(Program { file_metadata: program.file_metadata.clone(), declarations })
}

fn specialize_external_verb(
    verb: &ExternalVerbDecl,
    instances: &[GenericInstance],
) -> Result<Option<ExternalVerbDecl>, NativeEmitError> {
    let instance = instances.iter().find(|instance| instance_matches_external(instance, verb));
    let Some(instance) = instance else {
        return Ok(None);
    };
    let substitution = TypeSubstitution::for_type(
        &verb.name,
        &verb.generic_parameters,
        &instance.arguments,
        verb.span,
    )
    .map_err(|error| NativeEmitError(format!("generic verb substitution failed: {error:?}")))?;
    Ok(Some(ExternalVerbDecl {
        is_open: verb.is_open,
        doc: verb.doc.clone(),
        unsafe_boundary: verb.unsafe_boundary,
        module_import: verb.module_import,
        abi: verb.abi,
        metadata: verb.metadata.clone(),
        name: verb.name.clone(),
        generic_parameters: Vec::new(),
        params: verb.params.iter().map(|param| specialize_param(param, &substitution)).collect(),
        return_type: verb
            .return_type
            .as_ref()
            .map(|return_type| specialize_return_type(return_type, &substitution)),
        span: verb.span,
    }))
}

fn specialize_verb(
    verb: &VerbDecl,
    instances: &[GenericInstance],
) -> Result<Option<VerbDecl>, NativeEmitError> {
    let instance = instances.iter().find(|instance| instance_matches_verb(instance, verb));
    let Some(instance) = instance else {
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
        doc: verb.doc.clone(),
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
    instance.name == verb.name
        && instance.arguments.len() == verb.generic_parameters.len()
        && instance.arguments.iter().all(|argument| {
            !verb.generic_parameters.iter().any(|parameter| parameter.name == argument.name)
        })
}

fn instance_matches_external(instance: &GenericInstance, verb: &ExternalVerbDecl) -> bool {
    instance.name == verb.name
        && instance.arguments.len() == verb.generic_parameters.len()
        && instance.arguments.iter().all(|argument| {
            !verb.generic_parameters.iter().any(|parameter| parameter.name == argument.name)
        })
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
        Stmt::OwnerDecl { .. } => specialize_owner_declaration(statement, substitution),
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
        Stmt::CompoundAssignment { target, operator, value, span } => Stmt::CompoundAssignment {
            target: specialize_compound_target(target, substitution),
            operator: *operator,
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

fn specialize_compound_target(
    target: &crate::ast::CompoundAssignmentTarget,
    substitution: &TypeSubstitution,
) -> crate::ast::CompoundAssignmentTarget {
    match target {
        crate::ast::CompoundAssignmentTarget::Identifier(name) => {
            crate::ast::CompoundAssignmentTarget::Identifier(name.clone())
        }
        crate::ast::CompoundAssignmentTarget::Field { object, field } => {
            crate::ast::CompoundAssignmentTarget::Field {
                object: specialize_expression(object, substitution),
                field: field.clone(),
            }
        }
        crate::ast::CompoundAssignmentTarget::Index { target, index } => {
            crate::ast::CompoundAssignmentTarget::Index {
                target: specialize_expression(target, substitution),
                index: specialize_expression(index, substitution),
            }
        }
    }
}

fn specialize_owner_declaration(statement: &Stmt, substitution: &TypeSubstitution) -> Stmt {
    let Stmt::OwnerDecl { role, name, ty, span, initializer } = statement else { unreachable!() };
    Stmt::OwnerDecl {
        role: role.clone(),
        name: name.clone(),
        ty: ty.clone(),
        initializer: specialize_expression(initializer, substitution),
        span: *span,
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
        Expr::Binary { .. } => specialize_binary(expression, substitution),
        Expr::Call { .. } => specialize_call(expression, substitution),
        Expr::MethodCall { .. } => specialize_method_call(expression, substitution),
        Expr::StructLit { .. } => specialize_struct_literal(expression, substitution),
        Expr::FieldAccess { .. } => specialize_field_access(expression, substitution),
        Expr::Case { .. } => specialize_case(expression, substitution),
        _ => expression.clone(),
    }
}

fn specialize_binary(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::Binary { left, operator, right, span } = expression else { unreachable!() };
    Expr::Binary {
        left: Box::new(specialize_expression(left, substitution)),
        operator: *operator,
        right: Box::new(specialize_expression(right, substitution)),
        span: *span,
    }
}

fn specialize_call(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::Call { callee, arguments, span } = expression else { unreachable!() };
    Expr::Call {
        callee: callee.clone(),
        arguments: specialize_arguments(arguments, substitution),
        span: *span,
    }
}

fn specialize_method_call(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::MethodCall { receiver, method, arguments, span } = expression else { unreachable!() };
    Expr::MethodCall {
        receiver: Box::new(specialize_expression(receiver, substitution)),
        method: method.clone(),
        arguments: specialize_arguments(arguments, substitution),
        span: *span,
    }
}

fn specialize_struct_literal(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::StructLit { name, type_arguments, fields, span } = expression else { unreachable!() };
    Expr::StructLit {
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
    }
}

fn specialize_field_access(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::FieldAccess { object, field, span } = expression else { unreachable!() };
    Expr::FieldAccess {
        object: Box::new(specialize_expression(object, substitution)),
        field: field.clone(),
        span: *span,
    }
}

fn specialize_case(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::Case { mode, subject, branches, span } = expression else { unreachable!() };
    Expr::Case {
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
        reference_role: None,
        span: crate::lexer::SourceSpan::new(0, 0),
    })
}

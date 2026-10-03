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
    let instances = expand_transitive_instances(program, instances)?;
    let mut declarations = Vec::with_capacity(program.declarations.len());
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) if !verb.generic_parameters.is_empty() => {
                if let Some(specialized) = specialize_verb(verb, &instances)? {
                    declarations.push(TopLevelDecl::Verb(specialized));
                }
            }
            TopLevelDecl::ExternalVerb(verb) if !verb.generic_parameters.is_empty() => {
                if let Some(specialized) = specialize_external_verb(verb, &instances)? {
                    declarations.push(TopLevelDecl::ExternalVerb(specialized));
                }
            }
            other => declarations.push(other.clone()),
        }
    }
    Ok(Program { file_metadata: program.file_metadata.clone(), declarations })
}

fn expand_transitive_instances(
    program: &Program,
    instances: &[GenericInstance],
) -> Result<Vec<GenericInstance>, NativeEmitError> {
    let mut expanded = instances.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for caller_instance in expanded.clone() {
            let Some(caller_parameters) = generic_parameters_for(program, &caller_instance.name)
            else {
                continue;
            };
            let substitution = TypeSubstitution::for_type(
                &caller_instance.name,
                caller_parameters,
                &caller_instance.arguments,
                crate::lexer::SourceSpan::new(0, 0),
            )
            .map_err(|error| {
                NativeEmitError(format!("generic instance expansion failed: {error:?}"))
            })?;
            for nested_template in expanded.clone().into_iter().filter(|instance| {
                instance.caller.as_deref() == Some(caller_instance.name.as_str())
            }) {
                let arguments = nested_template
                    .arguments
                    .iter()
                    .map(|argument| substitution.apply(argument))
                    .collect::<Vec<_>>();
                if arguments
                    .iter()
                    .any(|argument| contains_generic_parameter(argument, caller_parameters))
                {
                    continue;
                }
                let canonical_key = format!(
                    "{}[{}]",
                    nested_template.name,
                    arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
                );
                if expanded.iter().any(|instance| instance.canonical_key == canonical_key) {
                    continue;
                }
                expanded.push(GenericInstance {
                    name: nested_template.name,
                    arguments,
                    canonical_key,
                    caller: Some(caller_instance.name.clone()),
                });
                changed = true;
            }
        }
    }
    Ok(expanded)
}

fn generic_parameters_for<'a>(
    program: &'a Program,
    name: &str,
) -> Option<&'a [crate::ast::GenericParam]> {
    program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Verb(verb) if verb.name == name => Some(verb.generic_parameters.as_slice()),
        TopLevelDecl::ExternalVerb(verb) if verb.name == name => {
            Some(verb.generic_parameters.as_slice())
        }
        _ => None,
    })
}

fn contains_generic_parameter(
    type_name: &TypeName,
    parameters: &[crate::ast::GenericParam],
) -> bool {
    parameters.iter().any(|parameter| parameter.name == type_name.name)
        || type_name
            .arguments
            .iter()
            .any(|argument| contains_generic_parameter(argument, parameters))
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
        Stmt::Assignment { target, value, span } => Stmt::Assignment {
            target: specialize_place(target, substitution),
            value: specialize_expression(value, substitution),
            span: *span,
        },
        Stmt::CompoundAssignment { target, operator, value, span } => Stmt::CompoundAssignment {
            target: specialize_place(target, substitution),
            operator: *operator,
            value: specialize_expression(value, substitution),
            span: *span,
        },
        Stmt::Expression { expression, span } => Stmt::Expression {
            expression: specialize_expression(expression, substitution),
            span: *span,
        },
        Stmt::If { condition, then_branch, else_branch, span } => Stmt::If {
            condition: specialize_expression(condition, substitution),
            then_branch: specialize_block(then_branch, substitution),
            else_branch: else_branch
                .as_ref()
                .map(|branch| specialize_if_branch(branch, substitution)),
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

fn specialize_place(
    target: &crate::ast::Place,
    substitution: &TypeSubstitution,
) -> crate::ast::Place {
    match target {
        crate::ast::Place::Binding { name, span } => {
            crate::ast::Place::Binding { name: name.clone(), span: *span }
        }
        crate::ast::Place::Field { object, field, span } => crate::ast::Place::Field {
            object: Box::new(specialize_place(object, substitution)),
            field: field.clone(),
            span: *span,
        },
        crate::ast::Place::Index { target, index, span } => crate::ast::Place::Index {
            target: Box::new(specialize_place(target, substitution)),
            index: specialize_expression(index, substitution),
            span: *span,
        },
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

fn specialize_if_branch(
    branch: &crate::ast::IfBranch,
    substitution: &TypeSubstitution,
) -> crate::ast::IfBranch {
    match branch {
        crate::ast::IfBranch::Block(block) => {
            crate::ast::IfBranch::Block(specialize_block(block, substitution))
        }
        crate::ast::IfBranch::ElseIf(expression) => {
            crate::ast::IfBranch::ElseIf(Box::new(specialize_expression(expression, substitution)))
        }
    }
}

fn specialize_expression(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    match expression {
        Expr::Identifier { name, span } => substitution.apply_const(name).map_or_else(
            || expression.clone(),
            |value| Expr::Integer { value: value.to_owned(), suffix: None, span: *span },
        ),
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
        Expr::Cast { expression: child, target, span } => Expr::Cast {
            expression: Box::new(specialize_expression(child, substitution)),
            target: substitution.apply(target),
            span: *span,
        },
        Expr::Binary { .. }
        | Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. }
        | Expr::Index { .. }
        | Expr::If { .. }
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
        Expr::Index { target, index, span } => Expr::Index {
            target: Box::new(specialize_expression(target, substitution)),
            index: Box::new(specialize_expression(index, substitution)),
            span: *span,
        },
        Expr::If { condition, then_branch, else_branch, span } => Expr::If {
            condition: Box::new(specialize_expression(condition, substitution)),
            then_branch: specialize_block(then_branch, substitution),
            else_branch: else_branch
                .as_ref()
                .map(|branch| specialize_if_branch(branch, substitution)),
            span: *span,
        },
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
        callee: callee.split_once('[').map_or_else(|| callee.clone(), |(name, _)| name.to_owned()),
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

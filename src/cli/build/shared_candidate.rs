use crate::ast::{
    BinaryOp, Block, DispatchMode, EnumPayload, Expr, IfBranch, Place, Program, Role, Stmt,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "shared_candidate_arrays.rs"]
mod shared_candidate_arrays;

use shared_candidate_arrays::{
    aggregate_array_bindings, aggregate_array_expression, array_bindings, fixed_array_expression,
    fixed_array_index_expression, integer_binding, scalar_place, supported_array_type,
};

#[path = "shared_candidate_cases.rs"]
mod shared_candidate_cases;

use shared_candidate_cases::{
    result_case_expression, scalar_case_expression, scalar_result_constructor_expression,
    scalar_statement_call,
};

#[path = "shared_candidate_types.rs"]
mod shared_candidate_types;

use shared_candidate_types::{supported_perform, unit_structs};

pub(super) fn supports_scalar_shared_lowering(program: &Program) -> bool {
    let unit_enums = unit_enums(program);
    let unit_structs = unit_structs(program);
    let returns_value = returns_value(program);
    program.declarations.iter().all(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => {
            let array_bindings = array_bindings(verb, &unit_structs);
            let aggregate_array_bindings = aggregate_array_bindings(verb, &unit_structs);
            let context = CandidateContext {
                returns_value: &returns_value,
                array_bindings: &array_bindings,
                aggregate_array_bindings: &aggregate_array_bindings,
                aggregate_types: &unit_structs,
            };
            verb.generic_parameters.is_empty()
                && verb
                    .params
                    .iter()
                    .all(|parameter| scalar_parameter(parameter, &unit_enums, &unit_structs))
                && verb
                    .return_type
                    .as_ref()
                    .is_none_or(|return_type| scalar_or_result_type(&return_type.ty, &unit_enums))
                && scalar_block(&verb.body, &context, true)
        }
        crate::ast::TopLevelDecl::ExternalVerb(verb) => {
            verb.generic_parameters.is_empty()
                && verb
                    .params
                    .iter()
                    .all(|parameter| scalar_parameter(parameter, &unit_enums, &unit_structs))
                && verb
                    .return_type
                    .as_ref()
                    .is_none_or(|return_type| scalar_or_result_type(&return_type.ty, &unit_enums))
        }
        crate::ast::TopLevelDecl::Enum(definition) => {
            definition.generic_parameters.is_empty()
                && definition
                    .variants
                    .iter()
                    .all(|variant| matches!(variant.payload, EnumPayload::Unit))
        }
        crate::ast::TopLevelDecl::Struct(definition) => unit_structs.contains(&definition.name),
        crate::ast::TopLevelDecl::Perform(perform) => {
            supported_perform(perform, program, &unit_structs)
        }
        _ => false,
    })
}

struct CandidateContext<'a> {
    returns_value: &'a BTreeMap<String, bool>,
    array_bindings: &'a BTreeSet<String>,
    aggregate_array_bindings: &'a BTreeSet<String>,
    aggregate_types: &'a BTreeSet<String>,
}

fn unit_enums(program: &Program) -> BTreeSet<String> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            crate::ast::TopLevelDecl::Enum(definition)
                if definition.generic_parameters.is_empty()
                    && definition
                        .variants
                        .iter()
                        .all(|variant| matches!(variant.payload, EnumPayload::Unit)) =>
            {
                Some(definition.name.clone())
            }
            _ => None,
        })
        .collect()
}

fn returns_value(program: &Program) -> BTreeMap<String, bool> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            crate::ast::TopLevelDecl::Verb(verb) => {
                Some((verb.name.clone(), verb.return_type.is_some()))
            }
            crate::ast::TopLevelDecl::ExternalVerb(verb) => {
                Some((verb.name.clone(), verb.return_type.is_some()))
            }
            _ => None,
        })
        .collect()
}

fn scalar_parameter(
    parameter: &crate::ast::Param,
    unit_enums: &BTreeSet<String>,
    unit_structs: &BTreeSet<String>,
) -> bool {
    matches!(parameter.role, Role::Erg | Role::Abs | Role::Ins | Role::Dat)
        && parameter.dispatch == DispatchMode::Static
        && (scalar_or_result_type(&parameter.ty, unit_enums)
            || supported_array_type(&parameter.ty, unit_structs)
            || unit_structs.contains(&parameter.ty.name))
}

fn scalar_type(type_name: &crate::ast::TypeName) -> bool {
    matches!(
        type_name.name.as_str(),
        "Bool" | "Int" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64"
    ) || matches!(type_name.name.as_str(), "Arena" | "Buffer" | "Region" | "String")
}

pub(super) fn discardable_result_payload_type(type_name: &crate::ast::TypeName) -> bool {
    matches!(
        type_name.name.as_str(),
        "Bool" | "Int" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64"
    )
}

fn scalar_option_type(type_name: &crate::ast::TypeName) -> bool {
    type_name.name == "Option"
        && type_name.arguments.len() == 1
        && type_name.arguments[0].reference_role == Some(Role::Abs)
        && type_name.arguments[0].arguments.is_empty()
}

fn scalar_or_result_type(type_name: &crate::ast::TypeName, unit_enums: &BTreeSet<String>) -> bool {
    scalar_type(type_name)
        || (type_name.name == "Result"
            && type_name.arguments.len() == 2
            && scalar_type(&type_name.arguments[0])
            && (scalar_type(&type_name.arguments[1])
                || unit_enums.contains(&type_name.arguments[1].name)))
}

fn scalar_block(block: &Block, context: &CandidateContext<'_>, allow_assignment: bool) -> bool {
    block.statements.iter().all(|statement| scalar_statement(statement, context, allow_assignment))
}

fn scalar_statement(
    statement: &Stmt,
    context: &CandidateContext<'_>,
    allow_assignment: bool,
) -> bool {
    match statement {
        Stmt::OwnerDecl { initializer, .. } => {
            scalar_expression(initializer, context)
                || scalar_try_expression(initializer, context)
                || value_if_expression(initializer, context)
        }
        Stmt::Assignment { target, value, .. } if allow_assignment => {
            assignment_place(target, context) && scalar_assignment_value(target, value, context)
        }
        Stmt::CompoundAssignment { target, value, .. } if allow_assignment => {
            scalar_assignment_place(target, context) && scalar_expression(value, context)
        }
        Stmt::Assignment { .. } | Stmt::CompoundAssignment { .. } => false,
        Stmt::Expression { expression, .. } => {
            (matches!(expression, Expr::Call { .. }) && scalar_statement_call(expression, context))
                || scalar_case_expression(expression, context)
                || scalar_result_constructor_expression(expression, context)
        }
        Stmt::If { .. } => scalar_if_statement(statement, context),
        Stmt::Return { value, .. } => {
            value.as_ref().is_none_or(|value| scalar_return_expression(value, context))
        }
        Stmt::Loop(block) => scalar_block(block, context, true),
        Stmt::Block(block) => scalar_block(block, context, true),
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::ForRange { binding, start, end, body, .. } => {
            integer_binding(binding)
                && scalar_expression(start, context)
                && scalar_expression(end, context)
                && range_loop_block(body, context)
        }
        Stmt::ForArray { binding, collection, body, .. } => {
            integer_binding(binding)
                && scalar_expression(collection, context)
                && array_loop_block(body, context)
        }
        Stmt::Drop { .. } => true,
    }
}

fn scalar_if_statement(statement: &Stmt, context: &CandidateContext<'_>) -> bool {
    let Stmt::If { condition, then_branch, else_branch, .. } = statement else { return false };
    scalar_expression(condition, context)
        && scalar_block(then_branch, context, true)
        && scalar_if_branch(else_branch.as_ref(), context)
}

fn scalar_if_branch(branch: Option<&IfBranch>, context: &CandidateContext<'_>) -> bool {
    match branch {
        None => true,
        Some(IfBranch::Block(block)) => scalar_block(block, context, true),
        Some(IfBranch::ElseIf(expression)) => scalar_if_expression(expression, context),
    }
}

fn scalar_if_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    let Expr::If { condition, then_branch, else_branch, .. } = expression else { return false };
    scalar_expression(condition, context)
        && scalar_block(then_branch, context, true)
        && scalar_if_branch(else_branch.as_ref(), context)
}

fn scalar_return_block(block: &Block, context: &CandidateContext<'_>) -> bool {
    scalar_block(block, context, false)
        && matches!(
            block.statements.last(),
            Some(Stmt::Return { value, .. })
                if value.as_ref().is_none_or(|value| {
                    scalar_expression(value, context) || result_case_expression(value, context)
                })
        )
}

fn scalar_assignment_place(place: &Place, context: &CandidateContext<'_>) -> bool {
    match place {
        Place::Binding { .. } => true,
        Place::Field { .. } => scalar_expression(&place.to_expr(), context),
        Place::Index { target, index, .. } => {
            fixed_array_expression(&target.to_expr(), context)
                && fixed_array_index_expression(index)
                && !aggregate_array_expression(
                    &place.to_expr(),
                    context.aggregate_array_bindings,
                    context.aggregate_types,
                )
        }
    }
}

fn assignment_place(place: &Place, context: &CandidateContext<'_>) -> bool {
    scalar_assignment_place(place, context)
        || matches!(place, Place::Index { .. })
            && aggregate_array_expression(
                &place.to_expr(),
                context.aggregate_array_bindings,
                context.aggregate_types,
            )
}

fn scalar_assignment_value(target: &Place, value: &Expr, context: &CandidateContext<'_>) -> bool {
    let target_expression = target.to_expr();
    if aggregate_array_expression(
        &target_expression,
        context.aggregate_array_bindings,
        context.aggregate_types,
    ) {
        let Expr::StructLit { fields, .. } = value else { return false };
        return fields.iter().all(|field| scalar_expression(&field.value, context));
    }
    scalar_expression(value, context) || scalar_try_expression(value, context)
}

fn scalar_try_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    let Expr::Try { expression, .. } = expression else { return false };
    scalar_expression(expression, context)
}

fn scalar_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::BufferLiteral { .. }
        | Expr::StringLiteral { .. } => true,
        Expr::Borrow { expression, .. } => scalar_expression(expression, context),
        Expr::Grouping { expression, .. } => scalar_nested_expression(expression, context),
        Expr::Unary { expression, .. } => scalar_expression(expression, context),
        Expr::Cast { expression, target, .. } => {
            scalar_type(target) && scalar_expression(expression, context)
        }
        Expr::Binary { left, operator, right, .. } => {
            scalar_binary_expression(left, *operator, right, context)
        }
        Expr::Call { callee, arguments, .. } => scalar_call_expression(callee, arguments, context),
        Expr::Case { .. } => scalar_case_expression(expression, context),
        Expr::MethodCall { receiver, method, arguments, .. } => {
            let Expr::Identifier { name, .. } = receiver.as_ref() else { return false };
            name.starts_with("Result[")
                && matches!(method.as_str(), "Ok" | "Err")
                && arguments.len() == 1
                && scalar_expression(&arguments[0].expression, context)
        }
        Expr::StructLit { name, fields, .. } => {
            !name.is_empty() && fields.iter().all(|field| scalar_expression(&field.value, context))
        }
        Expr::FieldAccess { object, .. } => {
            scalar_expression(object, context)
                || aggregate_array_expression(
                    object,
                    context.aggregate_array_bindings,
                    context.aggregate_types,
                )
        }
        Expr::Index { target, index, .. } => {
            fixed_array_expression(target, context)
                && fixed_array_index_expression(index)
                && !aggregate_array_expression(
                    expression,
                    context.aggregate_array_bindings,
                    context.aggregate_types,
                )
        }
        _ => false,
    }
}

fn scalar_nested_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::If { .. } => value_if_expression(expression, context),
        Expr::Grouping { expression, .. } => scalar_nested_expression(expression, context),
        Expr::Binary { left, operator, right, .. } => {
            scalar_binary_expression(left, *operator, right, context)
        }
        _ => scalar_expression(expression, context),
    }
}

fn scalar_return_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    scalar_expression(expression, context)
        || scalar_try_expression(expression, context)
        || value_if_expression(expression, context)
}

fn value_if_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    let Expr::If { condition, then_branch, else_branch, .. } = expression else { return false };
    scalar_expression(condition, context)
        && value_if_branch(then_branch, context)
        && matches!(
            else_branch,
            Some(IfBranch::Block(block)) if value_if_branch(block, context)
        )
}

fn value_if_branch(block: &Block, context: &CandidateContext<'_>) -> bool {
    let Some(Stmt::Expression { expression, .. }) = block.statements.last() else {
        return false;
    };
    block.statements[..block.statements.len() - 1]
        .iter()
        .all(|statement| scalar_statement(statement, context, true))
        && scalar_expression(expression, context)
}

fn scalar_binary_expression(
    left: &Expr,
    operator: BinaryOp,
    right: &Expr,
    context: &CandidateContext<'_>,
) -> bool {
    matches!(
        operator,
        BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::LessThan
            | BinaryOp::LessEquals
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterEquals
            | BinaryOp::Equals
            | BinaryOp::NotEquals
    ) && scalar_nested_expression(left, context)
        && scalar_nested_expression(right, context)
}

fn scalar_call_expression(
    callee: &str,
    arguments: &[crate::ast::Argument],
    context: &CandidateContext<'_>,
) -> bool {
    let array_constructor = arguments.is_empty()
        && callee.starts_with("Array[")
        && crate::semantic::parse_type_name_key(callee, crate::lexer::SourceSpan::new(0, 0))
            .is_some_and(|type_name| supported_array_type(&type_name, context.aggregate_types));
    (context.returns_value.get(callee).copied().unwrap_or(false) || array_constructor)
        && arguments.iter().all(|argument| scalar_expression(&argument.expression, context))
}

fn array_loop_block(block: &Block, context: &CandidateContext<'_>) -> bool {
    block.statements.iter().all(|statement| match statement {
        Stmt::OwnerDecl { initializer, .. } => array_loop_expression(initializer, context),
        Stmt::Assignment { target, value, .. } | Stmt::CompoundAssignment { target, value, .. } => {
            scalar_place(target) && array_loop_expression(value, context)
        }
        Stmt::Return { value, .. } => {
            value.as_ref().is_none_or(|value| array_loop_expression(value, context))
        }
        Stmt::Expression { expression, .. } => scalar_statement_call(expression, context),
        Stmt::If { .. }
        | Stmt::Loop(_)
        | Stmt::Block(_)
        | Stmt::ForArray { .. }
        | Stmt::ForRange { .. } => scalar_statement(statement, context, true),
        Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Drop { .. } => true,
    })
}

fn range_loop_block(block: &Block, context: &CandidateContext<'_>) -> bool {
    block.statements.iter().all(|statement| match statement {
        Stmt::OwnerDecl { initializer, .. } => scalar_expression(initializer, context),
        Stmt::Assignment { target, value, .. } | Stmt::CompoundAssignment { target, value, .. } => {
            scalar_place(target) && scalar_expression(value, context)
        }
        Stmt::Return { value, .. } => {
            value.as_ref().is_none_or(|value| scalar_expression(value, context))
        }
        Stmt::Expression { expression, .. } => scalar_statement_call(expression, context),
        Stmt::If { .. } | Stmt::Loop(_) | Stmt::Block(_) | Stmt::ForArray { .. } => {
            scalar_statement(statement, context, true)
        }
        Stmt::ForRange { binding, start, end, body, .. } => {
            integer_binding(binding)
                && scalar_expression(start, context)
                && scalar_expression(end, context)
                && range_loop_block(body, context)
        }
        Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Drop { .. } => true,
    })
}

fn array_loop_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::Index { target, index, .. } => {
            fixed_array_expression(target, context)
                && scalar_expression(index, context)
                && !aggregate_array_expression(
                    expression,
                    context.aggregate_array_bindings,
                    context.aggregate_types,
                )
        }
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            array_loop_expression(expression, context)
        }
        Expr::Cast { expression, target, .. } => {
            scalar_type(target) && array_loop_expression(expression, context)
        }
        Expr::Binary { left, operator, right, .. } => {
            matches!(
                operator,
                BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder
                    | BinaryOp::ShiftLeft
                    | BinaryOp::ShiftRight
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LessThan
                    | BinaryOp::LessEquals
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterEquals
                    | BinaryOp::Equals
                    | BinaryOp::NotEquals
            ) && array_loop_expression(left, context)
                && array_loop_expression(right, context)
        }
        _ => scalar_expression(expression, context),
    }
}

#[cfg(test)]
#[path = "shared_candidate_tests.rs"]
mod tests;

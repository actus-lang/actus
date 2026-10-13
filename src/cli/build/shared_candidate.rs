use crate::ast::{
    BinaryOp, Block, CaseBody, DispatchMode, EnumPayload, Expr, IfBranch, Pattern, Place, Program,
    Role, Stmt,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "shared_candidate_arrays.rs"]
mod shared_candidate_arrays;

use shared_candidate_arrays::{
    array_bindings, fixed_array_expression, fixed_array_index_expression, fixed_array_type,
    local_array_bindings,
};

pub(super) fn supports_scalar_shared_lowering(program: &Program) -> bool {
    let unit_enums = unit_enums(program);
    let unit_structs = unit_structs(program);
    let returns_value = returns_value(program);
    program.declarations.iter().all(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => {
            let array_bindings = array_bindings(verb);
            let local_array_bindings = local_array_bindings(verb);
            let context = CandidateContext {
                returns_value: &returns_value,
                array_bindings: &array_bindings,
                local_array_bindings: &local_array_bindings,
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
        crate::ast::TopLevelDecl::Struct(definition) => {
            supported_struct(definition) || supported_value_struct(definition)
        }
        crate::ast::TopLevelDecl::Perform(perform) => supported_perform(perform, program),
        _ => false,
    })
}

struct CandidateContext<'a> {
    returns_value: &'a BTreeMap<String, bool>,
    array_bindings: &'a BTreeSet<String>,
    local_array_bindings: &'a BTreeSet<String>,
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

fn unit_structs(program: &Program) -> BTreeSet<String> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            crate::ast::TopLevelDecl::Struct(definition)
                if supported_struct(definition) || supported_value_struct(definition) =>
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

fn supported_struct(definition: &crate::ast::StructDef) -> bool {
    definition.generic_parameters.is_empty()
        && definition.fields.iter().all(|field| {
            matches!(field.role, crate::ast::StructFieldRole::Erg) && scalar_type(&field.ty)
        })
}

fn supported_value_struct(definition: &crate::ast::StructDef) -> bool {
    definition.generic_parameters.is_empty()
        && definition.fields.iter().all(|field| {
            matches!(field.role, crate::ast::StructFieldRole::Value) && scalar_type(&field.ty)
        })
}

fn supported_perform(perform: &crate::ast::PerformDecl, program: &Program) -> bool {
    perform.target.arguments.is_empty()
        && program.declarations.iter().any(|declaration| {
            matches!(
                declaration,
                crate::ast::TopLevelDecl::Struct(definition)
                    if definition.name == perform.target.name && supported_struct(definition)
            )
        })
}

fn scalar_parameter(
    parameter: &crate::ast::Param,
    unit_enums: &BTreeSet<String>,
    unit_structs: &BTreeSet<String>,
) -> bool {
    matches!(parameter.role, Role::Erg | Role::Abs | Role::Ins | Role::Dat)
        && parameter.dispatch == DispatchMode::Static
        && (scalar_or_result_type(&parameter.ty, unit_enums)
            || fixed_array_type(&parameter.ty)
            || unit_structs.contains(&parameter.ty.name))
}

fn scalar_type(type_name: &crate::ast::TypeName) -> bool {
    matches!(
        type_name.name.as_str(),
        "Bool" | "Int" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64"
    ) || matches!(type_name.name.as_str(), "Arena" | "Buffer" | "Region" | "String")
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
        Stmt::OwnerDecl { initializer, .. } => scalar_initializer_expression(initializer, context),
        Stmt::Assignment { target, value, .. } if allow_assignment => {
            scalar_assignment_place(target, context) && scalar_expression(value, context)
        }
        Stmt::CompoundAssignment { target, value, .. } if allow_assignment => {
            scalar_assignment_place(target, context) && scalar_expression(value, context)
        }
        Stmt::Assignment { .. } | Stmt::CompoundAssignment { .. } => false,
        Stmt::Expression { expression, .. } => {
            matches!(expression, Expr::Call { .. }) && scalar_statement_call(expression, context)
        }
        Stmt::If { condition, then_branch, else_branch, .. } => {
            scalar_expression(condition, context)
                && scalar_return_block(then_branch, context)
                && matches!(
                    else_branch,
                    Some(IfBranch::Block(block))
                        if scalar_return_block(block, context)
                )
        }
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

fn integer_binding(binding: &crate::ast::ForBinding) -> bool {
    matches!(
        binding.ty.as_deref(),
        None | Some("u8") | Some("u16") | Some("u32") | Some("u64") | Some("Int")
    )
}

fn scalar_place(place: &Place) -> bool {
    matches!(place, Place::Binding { .. } | Place::Field { .. })
}

fn scalar_assignment_place(place: &Place, context: &CandidateContext<'_>) -> bool {
    scalar_place(place)
        || matches!(
            place,
            Place::Index { target, index, .. }
                if local_array_expression(&target.to_expr(), context)
                    && fixed_array_index_expression(index)
        )
}

fn local_array_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::Identifier { name, .. } => context.local_array_bindings.contains(name),
        Expr::Call { .. } => fixed_array_expression(expression, context),
        _ => false,
    }
}

fn scalar_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::BufferLiteral { .. } => true,
        Expr::Grouping { expression, .. } => scalar_nested_expression(expression, context),
        Expr::Unary { expression, .. } => scalar_expression(expression, context),
        Expr::Cast { expression, target, .. } => {
            scalar_type(target) && scalar_expression(expression, context)
        }
        Expr::Binary { left, operator, right, .. } => {
            scalar_binary_expression(left, *operator, right, context)
        }
        Expr::Call { callee, arguments, .. } => scalar_call_expression(callee, arguments, context),
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
        Expr::FieldAccess { object, .. } => scalar_expression(object, context),
        Expr::Index { target, index, .. } => {
            fixed_array_expression(target, context) && fixed_array_index_expression(index)
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

fn scalar_initializer_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    scalar_expression(expression, context) || value_if_expression(expression, context)
}

fn scalar_return_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    scalar_expression(expression, context)
        || result_case_expression(expression, context)
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
            .is_some_and(|type_name| fixed_array_type(&type_name));
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
        Stmt::If { .. } | Stmt::Loop(_) | Stmt::Block(_) | Stmt::ForArray { .. } => {
            scalar_statement(statement, context, true)
        }
        Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Drop { .. } => true,
        Stmt::ForRange { .. } => false,
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
        Stmt::If { .. } | Stmt::Loop(_) | Stmt::Block(_) => {
            scalar_statement(statement, context, true)
        }
        Stmt::ForRange { binding, start, end, body, .. } => {
            integer_binding(binding)
                && scalar_expression(start, context)
                && scalar_expression(end, context)
                && range_loop_block(body, context)
        }
        Stmt::ForArray { .. } => false,
        Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Drop { .. } => true,
    })
}

fn array_loop_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::Index { target, index, .. } => {
            fixed_array_expression(target, context) && scalar_expression(index, context)
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

fn result_case_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    let Expr::Case { subject, branches, .. } = expression else { return false };
    if branches.is_empty() || !scalar_expression(subject, context) {
        return false;
    }
    branches.iter().enumerate().all(|(index, branch)| {
        if branch.guard.is_some() {
            return false;
        }
        match &branch.body {
            CaseBody::Expression(body) if scalar_expression(body, context) => {}
            CaseBody::Block(block) if scalar_return_block(block, context) => {}
            _ => return false,
        }
        match &branch.pattern {
            Pattern::Variant { variant, .. } => matches!(variant.as_str(), "Ok" | "Err"),
            Pattern::Wildcard { .. } => index + 1 == branches.len(),
            Pattern::Literal { .. } => false,
        }
    })
}

fn scalar_statement_call(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    let Expr::Call { callee, arguments, .. } = expression else {
        return false;
    };
    context.returns_value.contains_key(callee)
        && arguments.iter().all(|argument| {
            !matches!(argument.expression, Expr::Index { .. })
                && scalar_expression(&argument.expression, context)
        })
}

#[cfg(test)]
#[path = "shared_candidate_tests.rs"]
mod tests;

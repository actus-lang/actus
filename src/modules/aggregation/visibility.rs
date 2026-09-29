use std::collections::HashSet;

use crate::ast::{Block, CaseBody, Expr, Program, Stmt, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::types::ModuleError;
use super::unit::ModuleUnit;

pub(super) fn validate_import_visibility(
    program: &Program,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    let private_verbs = private_names(unit, "verb");
    let private_types = private_names(unit, "struct");
    for declaration in &program.declarations {
        let block = match declaration {
            TopLevelDecl::Verb(verb) => Some(&verb.body),
            TopLevelDecl::Perform(perform) => perform.methods.first().map(|method| &method.body),
            _ => None,
        };
        if let Some(block) = block {
            check_block(block, &private_verbs, &private_types, module_path, unit)?;
        }
        if let TopLevelDecl::Struct(structure) = declaration {
            for field in &structure.fields {
                check_type_name(&field.ty.name, &private_types, module_path, unit, field.span)?;
            }
        }
    }
    Ok(())
}

fn private_names(unit: &ModuleUnit, kind: &str) -> HashSet<String> {
    unit.implementation()
        .declarations
        .iter()
        .filter_map(|declaration| super::validation::export_identity(declaration))
        .filter(|(declaration_kind, name)| {
            *declaration_kind == kind && !unit.exports().contains(declaration_kind, name)
        })
        .map(|(_, name)| name)
        .collect()
}

fn check_block(
    block: &Block,
    private_verbs: &HashSet<String>,
    private_types: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    for statement in &block.statements {
        match statement {
            Stmt::OwnerDecl { initializer, ty, span, .. } => {
                if let Some(type_name) = ty {
                    check_type_name(type_name, private_types, module_path, unit, *span)?;
                }
                check_expr(initializer, private_verbs, private_types, module_path, unit)?;
            }
            Stmt::Assignment { value, .. }
            | Stmt::Expression { expression: value, .. }
            | Stmt::Return { value: Some(value), .. } => {
                check_expr(value, private_verbs, private_types, module_path, unit)?;
            }
            Stmt::FieldAssignment { object, value, .. } => {
                check_expr(object, private_verbs, private_types, module_path, unit)?;
                check_expr(value, private_verbs, private_types, module_path, unit)?;
            }
            Stmt::IndexAssignment { target, index, value, .. } => {
                check_expr(target, private_verbs, private_types, module_path, unit)?;
                check_expr(index, private_verbs, private_types, module_path, unit)?;
                check_expr(value, private_verbs, private_types, module_path, unit)?;
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Loop(nested) | Stmt::Block(nested) => {
                check_block(nested, private_verbs, private_types, module_path, unit)?;
            }
            Stmt::Drop { .. } => {}
        }
    }
    Ok(())
}

fn check_expr(
    expression: &Expr,
    private_verbs: &HashSet<String>,
    private_types: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    match expression {
        Expr::Call { callee, span, arguments } => {
            reject_private(callee, private_verbs, module_path, unit, *span)?;
            for argument in arguments {
                check_expr(&argument.expression, private_verbs, private_types, module_path, unit)?;
            }
        }
        Expr::StructLit { name, span, fields, .. } => {
            reject_private(name, private_types, module_path, unit, *span)?;
            for field in fields {
                check_expr(&field.value, private_verbs, private_types, module_path, unit)?;
            }
        }
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. } => {
            check_expr(expression, private_verbs, private_types, module_path, unit)?;
        }
        Expr::Cast { expression, target, span } => {
            check_type_name(&target.name, private_types, module_path, unit, *span)?;
            check_expr(expression, private_verbs, private_types, module_path, unit)?;
        }
        Expr::Binary { left, right, .. } => {
            check_expr(left, private_verbs, private_types, module_path, unit)?;
            check_expr(right, private_verbs, private_types, module_path, unit)?;
        }
        Expr::MethodCall { receiver, arguments, .. } => {
            check_expr(receiver, private_verbs, private_types, module_path, unit)?;
            for argument in arguments {
                check_expr(&argument.expression, private_verbs, private_types, module_path, unit)?;
            }
        }
        Expr::FieldAccess { object, .. } | Expr::Index { target: object, .. } => {
            check_expr(object, private_verbs, private_types, module_path, unit)?;
            if let Expr::Index { index, .. } = expression {
                check_expr(index, private_verbs, private_types, module_path, unit)?;
            }
        }
        Expr::Case { subject, branches, .. } => {
            check_expr(subject, private_verbs, private_types, module_path, unit)?;
            check_case_branches(branches, private_verbs, private_types, module_path, unit)?;
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => {}
    }
    Ok(())
}

fn check_case_branches(
    branches: &[crate::ast::CaseBranch],
    private_verbs: &HashSet<String>,
    private_types: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    for branch in branches {
        if let Some(guard) = &branch.guard {
            check_expr(guard, private_verbs, private_types, module_path, unit)?;
        }
        match &branch.body {
            CaseBody::Expression(value) => {
                check_expr(value, private_verbs, private_types, module_path, unit)?;
            }
            CaseBody::Block(block) => {
                check_block(block, private_verbs, private_types, module_path, unit)?;
            }
        }
    }
    Ok(())
}

fn check_type_name(
    name: &str,
    private_types: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
    span: SourceSpan,
) -> Result<(), ModuleError> {
    reject_private(name, private_types, module_path, unit, span)
}

fn reject_private(
    name: &str,
    private_names: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
    span: SourceSpan,
) -> Result<(), ModuleError> {
    if private_names.contains(name) {
        return Err(ModuleError::PrivateDeclarationAccess {
            module: module_path.to_owned(),
            symbol: name.to_owned(),
            facade: unit.facade().to_owned(),
            span,
        });
    }
    Ok(())
}

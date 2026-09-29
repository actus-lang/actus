use crate::ast::{Block, BuiltinType, Expr, Stmt, TypeName};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

impl Analyzer {
    pub(super) fn require_buffer_length(&self, expression: &Expr) -> Result<(), SemanticError> {
        if self.expression_type(expression) == Some(BuiltinType::Int) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidIntrinsicArgument {
                callee: "Buffer".to_owned(),
                parameter: "length".to_owned(),
            },
            span: expression_span(expression),
        })
    }
}

pub(crate) fn try_type_mismatch(
    span: SourceSpan,
    expected: &TypeName,
    found: &str,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::TypeMismatch {
            callee: "?".to_owned(),
            parameter: "Result".to_owned(),
            expected: canonical_type_name(expected),
            found: found.to_owned(),
        },
        span,
    }
}

pub(crate) fn canonical_type_name(type_name: &TypeName) -> String {
    let role = type_name
        .reference_role
        .as_ref()
        .map(|role| match role {
            crate::ast::Role::Abs => "abs ",
            crate::ast::Role::Ins => "ins ",
            crate::ast::Role::Erg => "erg ",
            crate::ast::Role::Dat => "dat ",
        })
        .unwrap_or("");
    if type_name.arguments.is_empty() {
        return format!("{role}{}", type_name.name);
    }
    format!(
        "{role}{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

pub(super) fn block_guarantees_return(block: &Block) -> bool {
    block.statements.iter().any(statement_guarantees_return)
}

pub(crate) fn is_origin_return_expression(expression: &Expr) -> bool {
    match expression {
        Expr::Borrow { expression, .. } | Expr::Grouping { expression, .. } => {
            is_origin_return_expression(expression)
        }
        Expr::Call { .. } | Expr::MethodCall { .. } => true,
        _ => false,
    }
}

fn statement_guarantees_return(statement: &Stmt) -> bool {
    match statement {
        Stmt::Return { .. } => true,
        Stmt::Loop(block) => loop_guarantees_return(block),
        Stmt::Block(block) => block_guarantees_return(block),
        _ => false,
    }
}

fn loop_guarantees_return(block: &Block) -> bool {
    block_guarantees_return(block) && !contains_loop_exit(block)
}

fn contains_loop_exit(block: &Block) -> bool {
    block.statements.iter().any(|statement| match statement {
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::Block(nested) => contains_loop_exit(nested),
        Stmt::Loop(_) => false,
        _ => false,
    })
}

pub(crate) fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. } => *span,
    }
}

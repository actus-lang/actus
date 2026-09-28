use crate::ast::{Expr, PrimitiveType, Role, primitive_type};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::super::state::OwnershipState;

impl Analyzer {
    pub(crate) fn visit_return(
        &mut self,
        expression: Option<&Expr>,
        statement_span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expression) = expression else {
            if self.current_return_type.is_some() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MissingReturnValue,
                    span: statement_span,
                });
            }
            self.plan_return_unwind(statement_span);
            return Ok(());
        };
        let expected_return = self.current_return_type_name.clone();
        self.visit_expression_with_expected(expression, expected_return.as_ref())?;
        self.validate_return_type(expression)?;
        self.reject_arena_escape(expression)?;
        if self.current_return_access == Some(crate::ast::ReturnAccess::Abs) {
            self.validate_abs_return(expression)?;
            self.plan_return_unwind(statement_span);
            return Ok(());
        }
        let returned_expression = unwrap_grouping(expression);
        let Expr::Identifier { name, span } = returned_expression else {
            if contains_returned_borrow(expression) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BorrowedReturn { name: "temporary borrow".to_owned() },
                    span: expression_span(expression),
                });
            }
            self.plan_return_unwind(statement_span);
            return Ok(());
        };
        let index = self.binding(name, *span)?;
        self.ensure_access_available(index, name, *span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::BorrowedReturn { name: name.clone() },
                span: *span,
            });
        }
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::MoveFrozen {
                    name: name.clone(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span: *span,
            });
        }
        if matches!(self.model.bindings[index].ownership, OwnershipState::Active) {
            self.model.bindings[index].ownership = OwnershipState::Moved;
        }
        self.plan_return_unwind(statement_span);
        Ok(())
    }

    fn reject_arena_escape(&self, expression: &Expr) -> Result<(), SemanticError> {
        if self.arena_provenances(expression).is_empty() {
            return Ok(());
        }
        let name = match unwrap_grouping(expression) {
            Expr::Identifier { name, .. } => name.clone(),
            _ => "arena-derived value".to_owned(),
        };
        if name.is_empty() {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::ArenaReferenceEscape { name },
            span: expression_span(expression),
        })
    }

    fn validate_return_type(&self, expression: &Expr) -> Result<(), SemanticError> {
        if let Some(expected) = self.current_return_type_name.as_ref()
            && expected.name == "Result"
        {
            let valid = if matches!(expression, Expr::Try { .. }) {
                expected
                    .arguments
                    .first()
                    .and_then(|type_name| crate::ast::lookup_builtin_type(&type_name.name))
                    == self.expression_type(expression)
            } else if let Expr::Case { branches, .. } = expression {
                branches.iter().all(|branch| self.case_branch_returns_result(branch, expected))
            } else {
                let found = self.expression_type_name(expression);
                found.as_deref()
                    == Some(super::super::analyzer::canonical_type_name(expected).as_str())
                    || (expected.name == "Result" && found.as_deref() == Some("Result"))
            };
            if valid {
                return Ok(());
            }
            return Err(SemanticError {
                kind: SemanticErrorKind::ReturnTypeMismatch {
                    expected: super::super::analyzer::canonical_type_name(expected),
                    found: self
                        .expression_type_name(expression)
                        .unwrap_or_else(|| "unknown".to_owned()),
                },
                span: expression_span(expression),
            });
        }
        let Some(expected) = self.current_return_type_name.as_ref() else { return Ok(()) };
        let expected_name = super::super::analyzer::canonical_type_name(expected);
        let found = self.expression_type_name(expression).unwrap_or_else(|| "unknown".to_owned());
        if return_value_matches(expected, &expected_name, &found, expression) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::ReturnTypeMismatch { expected: expected_name, found },
            span: expression_span(expression),
        })
    }

    fn case_branch_returns_result(
        &self,
        branch: &crate::ast::CaseBranch,
        expected: &crate::ast::TypeName,
    ) -> bool {
        let expected = super::super::analyzer::canonical_type_name(expected);
        match &branch.body {
            crate::ast::CaseBody::Expression(expression) => {
                self.expression_type_name(expression).as_deref() == Some(expected.as_str())
            }
            crate::ast::CaseBody::Block(block) => {
                block.statements.iter().rev().find_map(|statement| {
                    let crate::ast::Stmt::Return { value: Some(expression), .. } = statement else {
                        return None;
                    };
                    Some(
                        self.expression_type_name(expression).as_deref() == Some(expected.as_str()),
                    )
                }) == Some(true)
            }
        }
    }
}

fn return_value_matches(
    expected: &crate::ast::TypeName,
    expected_name: &str,
    found: &str,
    expression: &Expr,
) -> bool {
    if strip_reference_role(expected_name) == strip_reference_role(found) {
        return true;
    }
    match primitive_type(&expected.name) {
        Some(PrimitiveType::Integer { .. }) => is_integer_literal(expression),
        Some(PrimitiveType::Float { .. }) => matches!(expression, Expr::FloatLiteral { .. }),
        Some(PrimitiveType::Void) | None => false,
    }
}

fn is_integer_literal(expression: &Expr) -> bool {
    match expression {
        Expr::Integer { .. } => true,
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            is_integer_literal(expression)
        }
        _ => false,
    }
}

fn strip_reference_role(type_name: &str) -> &str {
    type_name
        .strip_prefix("abs ")
        .or_else(|| type_name.strip_prefix("ins "))
        .or_else(|| type_name.strip_prefix("erg "))
        .or_else(|| type_name.strip_prefix("dat "))
        .unwrap_or(type_name)
}

fn contains_returned_borrow(expression: &Expr) -> bool {
    match expression {
        Expr::Borrow { .. } => true,
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            contains_returned_borrow(expression)
        }
        Expr::Try { expression, .. } => contains_returned_borrow(expression),
        Expr::Binary { left, right, .. } => {
            contains_returned_borrow(left) || contains_returned_borrow(right)
        }
        Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => false,
        Expr::StructLit { .. } | Expr::FieldAccess { .. } => false,
        Expr::Case { .. } => false,
    }
}

fn unwrap_grouping(mut expression: &Expr) -> &Expr {
    while let Expr::Grouping { expression: inner, .. } = expression {
        expression = inner;
    }
    expression
}

fn expression_span(expression: &Expr) -> SourceSpan {
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
        | Expr::Case { span, .. } => *span,
    }
}

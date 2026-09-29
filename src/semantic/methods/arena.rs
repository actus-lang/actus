use std::collections::HashSet;

use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::dispatch::expression_span;

impl Analyzer {
    pub(super) fn try_visit_arena_place(
        &mut self,
        receiver: &Expr,
        method: &str,
        arguments: &[crate::ast::Argument],
        span: SourceSpan,
    ) -> Option<Result<(), SemanticError>> {
        (method == "place")
            .then(|| self.arena_provenance(receiver))
            .flatten()
            .map(|arena_id| self.visit_arena_place(receiver, arena_id, arguments, span))
    }

    fn visit_arena_place(
        &mut self,
        receiver: &Expr,
        arena_id: usize,
        arguments: &[crate::ast::Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(receiver)?;
        if arguments.len() != 1 {
            return Err(SemanticError {
                kind: SemanticErrorKind::WrongArgumentCount { callee: "place".to_owned() },
                span,
            });
        }
        let argument = &arguments[0].expression;
        self.visit_expression(argument)?;
        let source_arenas = self.arena_provenances(argument);
        if source_arenas.iter().any(|source_arena| *source_arena != arena_id) {
            return Err(SemanticError {
                kind: SemanticErrorKind::CrossArenaReference { name: "placed value".to_owned() },
                span: expression_span(argument),
            });
        }
        let Some(type_name) = self.resolved_type_name(argument) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidReceiver { method: "place".to_owned() },
                span: expression_span(argument),
            });
        };
        self.inferred_expression_types.insert((span.start, span.end), type_name);
        self.expression_arena_provenance.insert((span.start, span.end), arena_id);
        Ok(())
    }

    pub(super) fn arena_provenance(&self, expression: &Expr) -> Option<usize> {
        self.arena_provenances(expression).into_iter().next()
    }

    pub(crate) fn arena_provenances(&self, expression: &Expr) -> HashSet<usize> {
        match expression {
            Expr::Identifier { name, span } => self
                .binding(name, *span)
                .ok()
                .into_iter()
                .flat_map(|index| self.binding_arena_provenance.get(&index).copied())
                .collect(),
            Expr::MethodCall { span, .. } | Expr::Call { span, .. } => self
                .expression_arena_provenance
                .get(&(span.start, span.end))
                .copied()
                .into_iter()
                .collect(),
            Expr::Grouping { expression, .. }
            | Expr::Borrow { expression, .. }
            | Expr::Unary { expression, .. }
            | Expr::Try { expression, .. } => self.arena_provenances(expression),
            Expr::Binary { left, right, .. } => self.binary_arena_provenances(left, right),
            Expr::StructLit { fields, .. } => {
                fields.iter().flat_map(|field| self.arena_provenances(&field.value)).collect()
            }
            Expr::FieldAccess { object, field, .. } => self
                .root_binding_index(object)
                .and_then(|index| self.field_arena_provenance.get(&(index, field.clone())).copied())
                .into_iter()
                .collect(),
            Expr::Index { target, .. }
                if self.expression_struct_type(expression).is_some()
                    || self
                        .resolved_type_name(expression)
                        .is_some_and(|type_name| type_name.name == "Array") =>
            {
                self.arena_provenances(target)
            }
            Expr::Index { .. } => HashSet::new(),
            Expr::Case { subject, branches, .. } => self.case_arena_provenances(subject, branches),
            _ => HashSet::new(),
        }
    }

    fn binary_arena_provenances(&self, left: &Expr, right: &Expr) -> HashSet<usize> {
        let mut arenas = self.arena_provenances(left);
        arenas.extend(self.arena_provenances(right));
        arenas
    }

    fn case_arena_provenances(
        &self,
        subject: &Expr,
        branches: &[crate::ast::CaseBranch],
    ) -> HashSet<usize> {
        let mut arenas = self.arena_provenances(subject);
        for branch in branches {
            self.extend_case_branch_arenas(&mut arenas, branch);
        }
        arenas
    }

    fn extend_case_branch_arenas(
        &self,
        arenas: &mut HashSet<usize>,
        branch: &crate::ast::CaseBranch,
    ) {
        match &branch.body {
            crate::ast::CaseBody::Expression(expression) => {
                arenas.extend(self.arena_provenances(expression));
            }
            crate::ast::CaseBody::Block(block) => {
                for statement in &block.statements {
                    if let crate::ast::Stmt::Return { value: Some(expression), .. } = statement {
                        arenas.extend(self.arena_provenances(expression));
                    }
                }
            }
        }
    }
}

use crate::ast::{Expr, TypeName};

use super::super::analyzer::Analyzer;

impl Analyzer {
    pub(crate) fn resolved_type_name(&self, expression: &Expr) -> Option<TypeName> {
        let type_name = if let Some(name) = self.literal_type_name(expression) {
            Some(TypeName::builtin(name, Vec::new(), None, literal_span(expression)))
        } else if let Expr::Call { callee, resolved_type: Some(type_name), .. } = expression
            && is_type_constructor_callee(callee)
        {
            Some(type_name.clone())
        } else if let Some(type_name) = self.inferred_call_type(expression) {
            Some(type_name)
        } else {
            self.fallback_expression_type(expression)
        };
        type_name.map(|type_name| self.normalize_imported_type_name(&type_name))
    }

    fn fallback_expression_type(&self, expression: &Expr) -> Option<TypeName> {
        match expression {
            Expr::StructLit { resolved_type, .. } => Some(resolved_type.clone().expect(
                "internal compiler error: struct literal reached semantic analysis without a resolved type",
            )),
            Expr::MethodCall { method, resolved_type, .. } => resolved_type.clone()
                .or_else(|| self.signatures.get(method).and_then(|s| s.return_type_name.clone()))
                .or_else(|| panic!("internal compiler error: method call reached semantic analysis without a resolved type")),
            Expr::Identifier { name, span } => self.identifier_type_name(name, *span),
            Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => self.resolved_type_name(expression),
            Expr::Cast { target, .. } => Some(target.clone()),
            Expr::Try { expression, .. } => self.enum_type_application(expression)
                .and_then(|result| result.arguments.first().cloned()),
            Expr::Call { callee, .. } => self.call_signature_name(callee)
                .and_then(|name| self.signatures.get(&name))
                .and_then(|signature| signature.return_type_name.clone()),
            Expr::Case { subject, branches, .. } => branches.iter().find_map(|branch| match &branch.body {
                crate::ast::CaseBody::Expression(expression)
                    if let Expr::Identifier { name, .. } = expression.as_ref()
                        && let Some(type_name) = self.pattern_binding_type(&branch.pattern, name, subject) =>
                {
                    Some(type_name)
                }
                crate::ast::CaseBody::Expression(expression) => self.resolved_type_name(expression),
                // A `return` leaves the verb; only a tail expression is the
                // value of a block branch.
                crate::ast::CaseBody::Block(block) => match block.statements.last() {
                    Some(crate::ast::Stmt::Expression { expression, .. }) => self.resolved_type_name(expression),
                    _ => None,
                },
            }),
            Expr::FieldAccess { object, resolved_type: Some(type_name), .. }
                if is_enum_qualifier(self, object) => Some(type_name.clone()),
            Expr::FieldAccess { object, field, .. } => self.resolved_type_name(object)
                .and_then(|type_name| self.specialized_field_type(&type_name, field)),
            Expr::Index { target, index, .. } => self.validate_index_access(target, index).ok(),
            _ => None,
        }
    }
}

fn is_enum_qualifier(analyzer: &Analyzer, object: &Expr) -> bool {
    match object {
        // `Option[T].None` spells the enum with its type application.
        Expr::Identifier { name, .. } => analyzer
            .enum_types
            .contains_key(name.split_once('[').map_or(name.as_str(), |(base, _)| base)),
        Expr::Index { target, .. } => {
            matches!(target.as_ref(), Expr::Identifier { name, .. } if analyzer.enum_types.contains_key(name))
        }
        _ => false,
    }
}

fn is_type_constructor_callee(callee: &str) -> bool {
    let Some((name, _)) = callee.split_once('[') else { return false };
    matches!(name, "Array" | "Arena" | "Buffer" | "Box" | "Map" | "Option" | "Result")
}

fn literal_span(expression: &Expr) -> crate::lexer::SourceSpan {
    match expression {
        Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::Integer { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. } => *span,
        _ => crate::lexer::SourceSpan::new(0, 0),
    }
}

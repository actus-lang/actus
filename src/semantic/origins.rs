use std::collections::BTreeSet;

use crate::ast::{Expr, ReturnAccess, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::{Origin, OriginRecord};

impl Analyzer {
    pub(super) fn initialize_origin_parameter_map(&mut self, parameters: &[crate::ast::Param]) {
        self.current_abs_origins.clear();
        for (index, parameter) in parameters.iter().enumerate() {
            if parameter.role == crate::ast::Role::Abs && is_view_source(&parameter.ty) {
                self.current_abs_origins.insert(parameter.name.clone(), index);
            }
        }
    }

    pub(super) fn origin_of(&self, expression: &Expr) -> Origin {
        match expression {
            Expr::Identifier { name, span } => self.binding_origin(name, *span),
            Expr::Borrow { expression, .. } | Expr::Grouping { expression, .. } => {
                self.origin_of(expression)
            }
            Expr::FieldAccess { object, .. } => derive_from(self.origin_of(object)),
            Expr::MethodCall { receiver, method, .. } => {
                if method == "raw_slice" {
                    return derive_from(self.origin_of(receiver));
                }
                let Some(signature) = self.signatures.get(method) else { return Origin::Unknown };
                if signature.return_access != Some(ReturnAccess::Abs) {
                    return Origin::None;
                }
                self.origin_from_abs_call(std::iter::once(receiver.as_ref()))
            }
            Expr::Call { callee, arguments, .. } => {
                let Some(signature) = self.signatures.get(callee) else { return Origin::Unknown };
                if signature.return_access != Some(ReturnAccess::Abs) {
                    return Origin::None;
                }
                self.origin_from_abs_call(arguments.iter().map(|argument| &argument.expression))
            }
            Expr::Binary { left, right, .. } => {
                combine([self.origin_of(left), self.origin_of(right)])
            }
            Expr::Unary { expression, .. } => self.origin_of(expression),
            Expr::Case { .. } => Origin::Unknown,
            Expr::BufferLiteral { .. }
            | Expr::Integer { .. }
            | Expr::FloatLiteral { .. }
            | Expr::StringLiteral { .. }
            | Expr::StructLit { .. } => Origin::None,
        }
    }

    pub(super) fn record_origin(&mut self, expression: &Expr) -> Origin {
        let origin = self.origin_of(expression);
        self.model
            .expression_origins
            .push(OriginRecord { span: expression_span(expression), origin: origin.clone() });
        origin
    }

    pub(super) fn validate_abs_return(&self, expression: &Expr) -> Result<(), SemanticError> {
        if self.current_abs_origins.len() != 1 {
            return Err(abs_origin_error(
                "an abs return requires exactly one non-scalar abs parameter",
                expression,
            ));
        }
        let expected = *self.current_abs_origins.values().next().expect("one origin");
        let origin = self.origin_of(expression);
        let valid = matches!(origin, Origin::AbsParameter { parameter_index } if parameter_index == expected)
            || matches!(origin, Origin::Derived { root_parameter } if root_parameter == expected);
        if valid {
            return Ok(());
        }
        let reason = match origin {
            Origin::Multiple { .. } => "the returned view has multiple origins",
            Origin::Unknown => "the returned view has an unknown origin",
            Origin::None => "the returned view has no abs parameter origin",
            _ => "the returned view does not preserve its abs parameter origin",
        };
        Err(abs_origin_error(reason, expression))
    }

    fn binding_origin(&self, name: &str, span: SourceSpan) -> Origin {
        let Some(index) = self.binding(name, span).ok() else { return Origin::Unknown };
        self.binding_origins.get(&index).cloned().unwrap_or_else(|| {
            self.current_abs_origins
                .get(name)
                .copied()
                .map_or(Origin::None, |parameter_index| Origin::AbsParameter { parameter_index })
        })
    }

    fn origin_from_abs_call<'a>(&self, expressions: impl Iterator<Item = &'a Expr>) -> Origin {
        combine(expressions.map(|expression| self.origin_of(expression)))
    }
}

fn is_view_source(type_name: &TypeName) -> bool {
    !matches!(type_name.name.as_str(), "Int" | "Bool" | "String")
}

fn derive_from(origin: Origin) -> Origin {
    match origin {
        Origin::AbsParameter { parameter_index }
        | Origin::Derived { root_parameter: parameter_index } => {
            Origin::Derived { root_parameter: parameter_index }
        }
        other => other,
    }
}

fn combine<I>(origins: I) -> Origin
where
    I: IntoIterator<Item = Origin>,
{
    let mut roots = BTreeSet::new();
    let mut saw_unknown = false;
    for origin in origins {
        match origin {
            Origin::AbsParameter { parameter_index }
            | Origin::Derived { root_parameter: parameter_index } => {
                roots.insert(parameter_index);
            }
            Origin::Unknown => saw_unknown = true,
            Origin::Multiple { roots: nested } => roots.extend(nested),
            Origin::None => {}
        }
    }
    if roots.len() > 1 {
        return Origin::Multiple { roots: roots.into_iter().collect() };
    }
    if saw_unknown {
        return Origin::Unknown;
    }
    roots.into_iter().next().map_or(Origin::None, |root| Origin::Derived { root_parameter: root })
}

fn abs_origin_error(reason: &str, expression: &Expr) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidAbsReturnOrigin { reason: reason.to_owned() },
        span: expression_span(expression),
    }
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
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Case { span, .. } => *span,
    }
}

use crate::ast::Expr;
use crate::lexer::SourceSpan;

pub(super) fn initializer_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Try { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. }
        | Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Cast { span, .. } => *span,
    }
}

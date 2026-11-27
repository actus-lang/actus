use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use crate::semantic::analyzer::Analyzer;
use crate::semantic::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    /// Rejects reading a struct, enum, pack, array or resource BY VALUE through
    /// a path whose root is an `abs` view or an `ins` loan (bug #108,
    /// ADR-0124). A view is not an owner, so the read cannot move the value
    /// out, and nothing writes a copy, so the value would be shared silently.
    /// The check covers every place that stores or hands over a value: a
    /// return, an `erg` initializer, an assignment, a struct literal field, an
    /// enum payload and an `erg` or `dat` argument. It fires before the
    /// field is recorded as moved, so the view itself never ends up marked
    /// as moved (bug #113).
    ///
    /// Only a field or element path is judged: a plain view binding has its
    /// own diagnostics (`E1018`, `E1019`, `E1010`, `E1082`), a scalar read is
    /// a value (ADR-0124, bug #93), and a read by view (`abs`, `ref`, `case`
    /// on an `abs` subject, a method with `abs self`) never reaches this check.
    /// The error says whether `copy(value: abs path)` is the fix, which holds
    /// when the type of the path is a copy type.
    pub(in crate::semantic) fn reject_move_out_of_view(
        &self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(path) = view_read_path(expression) else { return Ok(()) };
        let Some(index) = self.root_binding_index(expression) else { return Ok(()) };
        let loan = match self.model.bindings[index].role {
            Role::Abs => false,
            Role::Ins => true,
            _ => return Ok(()),
        };
        if !self.is_move_only_aggregate(expression) {
            return Ok(());
        }
        let copyable = self
            .resolved_type_name(expression)
            .is_some_and(|type_name| self.type_is_copy(&type_name));
        Err(SemanticError { kind: SemanticErrorKind::MoveOutOfView { path, loan, copyable }, span })
    }
}

/// The source spelling of a field or element path rooted at a binding, such as
/// `p.header` or `o.items[0].inner`; `None` for a plain binding or any other
/// expression. A non-literal index is written `[..]`.
fn view_read_path(expression: &Expr) -> Option<String> {
    match expression {
        Expr::FieldAccess { object, field, .. } => {
            Some(format!("{}.{field}", path_prefix(object)?))
        }
        Expr::Index { target, index, .. } => {
            let element = match index.as_ref() {
                Expr::Integer { value, suffix, .. } => {
                    format!("{value}{}", suffix.as_deref().unwrap_or(""))
                }
                _ => "..".to_owned(),
            };
            Some(format!("{}[{element}]", path_prefix(target)?))
        }
        _ => None,
    }
}

fn path_prefix(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Identifier { name, .. } => Some(name.clone()),
        _ => view_read_path(expression),
    }
}

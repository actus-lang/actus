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
            _ => return self.reject_move_out_of_element(expression, path, span),
        };
        if !self.is_move_only_aggregate(expression) {
            return Ok(());
        }
        let copyable = self
            .resolved_type_name(expression)
            .is_some_and(|type_name| self.type_is_copy(&type_name));
        Err(SemanticError { kind: SemanticErrorKind::MoveOutOfView { path, loan, copyable }, span })
    }

    /// Rejects reading a struct, enum, pack, array or resource BY VALUE out of
    /// an array element of an owner, directly (`xs[i]`) or through a field of
    /// the element (`xs[i].inner`), in every position that stores or hands
    /// over a value (bug #123, ADR-0124). The language does not track moves
    /// of single elements, so a move would leave the array releasing the
    /// element again at its own cleanup (bug #126), and a silent copy would
    /// share whatever the value owns. The element can be borrowed with `abs`,
    /// replaced in place by an assignment (ADR-0125), or, when its type is a
    /// copy type, duplicated with `copy(value: abs xs[i])`; the error says
    /// whether that copy is the fix. A scalar element is a value and is
    /// never judged.
    fn reject_move_out_of_element(
        &self,
        expression: &Expr,
        path: String,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if !passes_through_element(expression) || !self.is_move_only_aggregate(expression) {
            return Ok(());
        }
        let copyable = self
            .resolved_type_name(expression)
            .is_some_and(|type_name| self.type_is_copy(&type_name));
        Err(SemanticError { kind: SemanticErrorKind::MoveOutOfElement(path, copyable), span })
    }
}

/// Whether a field or element path reads from an array element anywhere
/// along it; an index into a `Buffer` yields a byte, which is never judged.
fn passes_through_element(expression: &Expr) -> bool {
    match expression {
        Expr::Index { .. } => true,
        Expr::FieldAccess { object, .. } => passes_through_element(object),
        _ => false,
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

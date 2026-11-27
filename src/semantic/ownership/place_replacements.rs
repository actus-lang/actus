//! Assignments to a field or an array element whose place holds an owned
//! value (ADR-0125, bug #126). The value the place held is released after the
//! assigned expression was evaluated and before the new value is stored, as
//! an assignment to a whole owner does. A field that was moved out before
//! holds nothing to release; the assignment makes it owned again.

use crate::ast::{Expr, TypeName};
use crate::lexer::SourceSpan;

use crate::semantic::analyzer::Analyzer;
use crate::semantic::cleanup::{PlaceReplacement, ReplacedPlace};
use crate::semantic::state::OwnershipState;

impl Analyzer {
    /// Whether the assignment to `object.field` gives a value back to a field
    /// of a partially moved owner: the owner keeps no moved path that the
    /// field lies inside, so the field itself, a path inside it or another
    /// field was moved. Such an assignment is accepted although the owner as
    /// a whole cannot be read; an assignment into a moved value is not.
    pub(in crate::semantic) fn assignment_restores_field(
        &self,
        index: usize,
        object: &Expr,
        field: &str,
    ) -> bool {
        let OwnershipState::PartiallyMoved { fields } = &self.model.bindings[index].ownership
        else {
            return false;
        };
        let Some(path) = field_path(object, field) else { return false };
        !fields.iter().any(|moved| path.starts_with(&format!("{moved}.")))
    }

    /// Records that the assignment at `span` to the owning field `field` of
    /// `object`, whose declared type is `field_type`, releases the value the
    /// field holds. It runs after the assigned expression was analysed, so a
    /// field that the expression moved (`c.b = grow(b: dat c.b)`) holds
    /// nothing any more and is not released. The field is owned again
    /// afterwards; the paths inside it that were moved are skipped by its
    /// release, and so is its own drop contract.
    pub(in crate::semantic) fn replace_owned_field(
        &mut self,
        object: &Expr,
        field: &str,
        field_type: &TypeName,
        span: SourceSpan,
    ) {
        let Some(index) = self.root_binding_index(object) else { return };
        let moved_fields = match field_path(object, field) {
            Some(path) => match self.restore_moved_path(index, &path) {
                Some(inside) => inside,
                None => return,
            },
            // A field of an array element: nothing can be moved out of an
            // element, so the field is always owned.
            None => Vec::new(),
        };
        if !self.type_holds_resource(field_type) {
            return;
        }
        let Some(owner_type) = self.resolved_type_name(object) else { return };
        self.model.place_replacements.push(PlaceReplacement {
            span,
            binding_index: index,
            place: ReplacedPlace::Field { owner_type, field: field.to_owned(), moved_fields },
        });
    }

    /// Records that the assignment at `span` to an element of `target`, an
    /// array of `element_type`, releases the element it replaces. An element
    /// is always owned: moving a value out of an element is rejected, so no
    /// element is ever empty.
    pub(in crate::semantic) fn replace_owned_element(
        &mut self,
        target: &Expr,
        element_type: &TypeName,
        span: SourceSpan,
    ) {
        let Some(index) = self.root_binding_index(target) else { return };
        if !self.type_holds_resource(element_type) {
            return;
        }
        self.model.place_replacements.push(PlaceReplacement {
            span,
            binding_index: index,
            place: ReplacedPlace::Element { element_type: element_type.clone() },
        });
    }

    /// Makes the field path `path` of the owner `index` owned again. Returns
    /// `None` when the path itself had been moved, so the field holds nothing
    /// to release, and otherwise the moved paths inside it, relative to it.
    fn restore_moved_path(&mut self, index: usize, path: &str) -> Option<Vec<String>> {
        let OwnershipState::PartiallyMoved { fields } = &self.model.bindings[index].ownership
        else {
            return Some(Vec::new());
        };
        let inner = format!("{path}.");
        let whole = fields.iter().any(|moved| moved == path);
        let inside = fields
            .iter()
            .filter_map(|moved| moved.strip_prefix(&inner))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let remaining = fields
            .iter()
            .filter(|moved| moved.as_str() != path && !moved.starts_with(&inner))
            .cloned()
            .collect::<Vec<_>>();
        self.model.bindings[index].ownership = if remaining.is_empty() {
            OwnershipState::Active
        } else {
            OwnershipState::PartiallyMoved { fields: remaining }
        };
        (!whole).then_some(inside)
    }
}

/// The path of `object.field` inside its root binding (`inner.b` for
/// `c.inner.b`), or `None` when the path passes through an array element.
fn field_path(object: &Expr, field: &str) -> Option<String> {
    match object {
        Expr::Identifier { .. } => Some(field.to_owned()),
        Expr::FieldAccess { object: parent, field: parent_field, .. } => {
            Some(format!("{}.{field}", field_path(parent, parent_field)?))
        }
        _ => None,
    }
}

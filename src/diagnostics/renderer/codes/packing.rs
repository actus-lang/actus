use crate::diagnostics::DiagnosticCode;
use crate::semantic::{PackLayoutError, SemanticErrorKind};

pub(super) fn code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::PackLayout(PackLayoutError::InvalidPackStorage { .. }) => {
            DiagnosticCode::INVALID_PACK_STORAGE
        }
        SemanticErrorKind::PackLayout(PackLayoutError::InvalidPackFieldType { .. }) => {
            DiagnosticCode::INVALID_PACK_FIELD_TYPE
        }
        SemanticErrorKind::PackLayout(PackLayoutError::PackFieldOutOfBounds { .. }) => {
            DiagnosticCode::PACK_FIELD_OUT_OF_BOUNDS
        }
        SemanticErrorKind::PackLayout(PackLayoutError::PackFieldOverlap { .. }) => {
            DiagnosticCode::PACK_FIELD_OVERLAP
        }
        SemanticErrorKind::PackLayout(PackLayoutError::PackUncoveredBits { .. }) => {
            DiagnosticCode::PACK_UNCOVERED_BITS
        }
        _ => return None,
    })
}

pub(super) fn message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::PackLayout(PackLayoutError::InvalidPackStorage { pack, ty }) => {
            format!("pack `{pack}` requires unsigned fixed storage, found `{ty}`")
        }
        SemanticErrorKind::PackLayout(PackLayoutError::InvalidPackFieldType {
            pack,
            field,
            ty,
        }) => {
            format!("pack field `{field}` in `{pack}` requires an integer type, found `{ty}`")
        }
        SemanticErrorKind::PackLayout(PackLayoutError::PackFieldOutOfBounds {
            pack,
            field,
            offset,
            width,
            capacity,
        }) => {
            format!(
                "pack field `{field}` in `{pack}` exceeds {capacity}-bit storage: offset {offset} + width {width}"
            )
        }
        SemanticErrorKind::PackLayout(PackLayoutError::PackFieldOverlap {
            pack,
            field,
            other,
            start,
            end,
        }) => {
            format!(
                "pack fields `{field}` and `{other}` in `{pack}` overlap at bits {start}..{end}"
            )
        }
        SemanticErrorKind::PackLayout(PackLayoutError::PackUncoveredBits { pack, start, end }) => {
            format!(
                "pack `{pack}` leaves bits {start}..{end} uncovered; add an explicit `_reserved` field"
            )
        }
        _ => return None,
    };
    Some(message)
}

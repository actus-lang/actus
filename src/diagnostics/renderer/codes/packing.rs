use crate::diagnostics::DiagnosticCode;
use crate::semantic::SemanticErrorKind;

pub(super) fn code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::InvalidPackStorage { .. } => DiagnosticCode::INVALID_PACK_STORAGE,
        SemanticErrorKind::InvalidPackFieldType { .. } => DiagnosticCode::INVALID_PACK_FIELD_TYPE,
        SemanticErrorKind::PackFieldOutOfBounds { .. } => DiagnosticCode::PACK_FIELD_OUT_OF_BOUNDS,
        SemanticErrorKind::PackFieldOverlap { .. } => DiagnosticCode::PACK_FIELD_OVERLAP,
        SemanticErrorKind::PackUncoveredBits { .. } => DiagnosticCode::PACK_UNCOVERED_BITS,
        _ => return None,
    })
}

pub(super) fn message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::InvalidPackStorage { pack, ty } => {
            format!("pack `{pack}` requires unsigned fixed storage, found `{ty}`")
        }
        SemanticErrorKind::InvalidPackFieldType { pack, field, ty } => {
            format!("pack field `{field}` in `{pack}` requires an integer type, found `{ty}`")
        }
        SemanticErrorKind::PackFieldOutOfBounds { pack, field, offset, width, capacity } => {
            format!(
                "pack field `{field}` in `{pack}` exceeds {capacity}-bit storage: offset {offset} + width {width}"
            )
        }
        SemanticErrorKind::PackFieldOverlap { pack, field, other, start, end } => {
            format!(
                "pack fields `{field}` and `{other}` in `{pack}` overlap at bits {start}..{end}"
            )
        }
        SemanticErrorKind::PackUncoveredBits { pack, start, end } => {
            format!(
                "pack `{pack}` leaves bits {start}..{end} uncovered; add an explicit `_reserved` field"
            )
        }
        _ => return None,
    };
    Some(message)
}

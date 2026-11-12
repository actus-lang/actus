use crate::semantic::SemanticErrorKind;

pub(super) fn code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::InvalidPackStorage { .. } => "E1070",
        SemanticErrorKind::InvalidPackFieldType { .. } => "E1071",
        SemanticErrorKind::PackFieldOutOfBounds { .. } => "E1072",
        SemanticErrorKind::PackFieldOverlap { .. } => "E1073",
        SemanticErrorKind::PackUncoveredBits { .. } => "E1074",
        SemanticErrorKind::InvalidPackFieldRole { .. } => "E1075",
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
        SemanticErrorKind::InvalidPackFieldRole { pack, field, role } => {
            format!("pack field `{field}` in `{pack}` has unsupported access role `{role}`")
        }
        _ => return None,
    };
    Some(message)
}

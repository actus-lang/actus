//! Pack-layout errors: invalid storage, field types and bit layout of a
//! `pack` declaration (ADR-0026).

/// Semantic errors raised while validating a pack's storage, field types and
/// bit layout (ADR-0026).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackLayoutError {
    InvalidPackStorage { pack: String, ty: String },
    InvalidPackFieldType { pack: String, field: String, ty: String },
    PackFieldOutOfBounds { pack: String, field: String, offset: u16, width: u16, capacity: u16 },
    PackFieldOverlap { pack: String, field: String, other: String, start: u16, end: u16 },
    PackUncoveredBits { pack: String, start: u16, end: u16 },
}

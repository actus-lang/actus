use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use crate::ast::{PackDecl, PackField, PrimitiveType, Program, Role, TopLevelDecl, primitive_type};

impl Analyzer {
    pub(super) fn validate_pack_declarations(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Pack(pack) = declaration else { continue };
            validate_pack(pack)?;
        }
        Ok(())
    }
}

fn validate_pack(pack: &PackDecl) -> Result<(), SemanticError> {
    let capacity = storage_capacity(pack)?;
    let mut intervals: Vec<(String, u16, u16)> = Vec::new();
    let mut coverage = vec![false; capacity as usize];
    let mut reserved = Vec::new();

    for field in &pack.fields {
        let width = field_width(pack, field)?;
        let end = field
            .offset
            .checked_add(width)
            .ok_or_else(|| out_of_bounds(pack, field, width, capacity))?;
        if end > capacity {
            return Err(out_of_bounds(pack, field, width, capacity));
        }
        if let Some((other, start, overlap_end)) =
            intervals.iter().find_map(|(name, start, other_end)| {
                let overlap_start = field.offset.max(*start);
                let overlap_end = end.min(*other_end);
                (overlap_start < overlap_end).then(|| (name.clone(), overlap_start, overlap_end))
            })
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::PackFieldOverlap {
                    pack: pack.name.clone(),
                    field: field.name.clone(),
                    other,
                    start,
                    end: overlap_end,
                },
                span: field.span,
            });
        }
        for bit in field.offset..end {
            coverage[bit as usize] = true;
        }
        if field.name == "_reserved" {
            reserved.push((field.offset, end));
        }
        intervals.push((field.name.clone(), field.offset, end));
    }

    validate_full_coverage(pack, &coverage, &reserved)
}

fn storage_capacity(pack: &PackDecl) -> Result<u16, SemanticError> {
    match primitive_type(&pack.storage.name) {
        Some(PrimitiveType::Integer { signed: false, width })
            if matches!(width, 8 | 16 | 32 | 64 | 128) =>
        {
            Ok(width as u16)
        }
        _ => Err(SemanticError {
            kind: SemanticErrorKind::InvalidPackStorage {
                pack: pack.name.clone(),
                ty: pack.storage.name.clone(),
            },
            span: pack.storage.span,
        }),
    }
}

fn field_width(pack: &PackDecl, field: &PackField) -> Result<u16, SemanticError> {
    match &field.role {
        Role::Erg | Role::Abs => {}
        role => {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidPackFieldRole {
                    pack: pack.name.clone(),
                    field: field.name.clone(),
                    role: role_name(role).to_owned(),
                },
                span: field.span,
            });
        }
    }
    match primitive_type(&field.ty.name) {
        Some(PrimitiveType::Integer { width, .. }) => Ok(width as u16),
        _ => Err(SemanticError {
            kind: SemanticErrorKind::InvalidPackFieldType {
                pack: pack.name.clone(),
                field: field.name.clone(),
                ty: field.ty.name.clone(),
            },
            span: field.ty.span,
        }),
    }
}

fn out_of_bounds(pack: &PackDecl, field: &PackField, width: u16, capacity: u16) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::PackFieldOutOfBounds {
            pack: pack.name.clone(),
            field: field.name.clone(),
            offset: field.offset,
            width: width as u8,
            capacity,
        },
        span: field.span,
    }
}

fn validate_full_coverage(
    pack: &PackDecl,
    coverage: &[bool],
    reserved: &[(u16, u16)],
) -> Result<(), SemanticError> {
    let mut bit = 0;
    while bit < coverage.len() {
        if coverage[bit] {
            bit += 1;
            continue;
        }
        let start = bit as u16;
        while bit < coverage.len() && !coverage[bit] {
            bit += 1;
        }
        let end = bit as u16;
        if !reserved
            .iter()
            .any(|(reserved_start, reserved_end)| *reserved_start <= start && *reserved_end >= end)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::PackUncoveredBits { pack: pack.name.clone(), start, end },
                span: pack.span,
            });
        }
    }
    Ok(())
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}

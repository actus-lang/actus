use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use crate::ast::{PackDecl, PackField, PrimitiveType, Program, Role, TopLevelDecl, primitive_type};

impl Analyzer {
    pub(super) fn validate_pack_declarations(
        &mut self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Pack(pack) = declaration else { continue };
            validate_pack(pack)?;
            if self.pack_types.insert(pack.name.clone(), pack.clone()).is_some() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicatePackName { name: pack.name.clone() },
                    span: pack.span,
                });
            }
        }
        Ok(())
    }

    pub(super) fn validate_pack_literal(
        &mut self,
        name: &str,
        fields: &[crate::ast::StructFieldInit],
        span: crate::lexer::SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(pack) = self.pack_types.get(name).cloned() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownType { name: name.to_owned() },
                span,
            });
        };
        let storage =
            fields.iter().find(|field| field.name == "storage").ok_or_else(|| SemanticError {
                kind: SemanticErrorKind::MissingStructField {
                    struct_name: name.to_owned(),
                    field: "storage".to_owned(),
                },
                span,
            })?;
        if fields.iter().filter(|field| field.name == "storage").count() != 1 {
            return Err(SemanticError {
                kind: SemanticErrorKind::DuplicateStructField {
                    struct_name: name.to_owned(),
                    field: "storage".to_owned(),
                },
                span: storage.span,
            });
        }
        if let Some(extra) = fields.iter().find(|field| field.name != "storage") {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name: name.to_owned(),
                    field: extra.name.clone(),
                },
                span: extra.span,
            });
        }
        self.visit_expression(&storage.value)?;
        self.validate_expected_literal(&storage.value, &pack.storage)
    }

    pub(super) fn pack_field(&self, pack: &str, field: &str) -> Option<&PackField> {
        self.pack_types.get(pack)?.fields.iter().find(|candidate| candidate.name == field)
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

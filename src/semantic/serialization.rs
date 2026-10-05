use crate::ast::{
    PrimitiveType, Program, SerializeDecl, SerializeSection, TopLevelDecl, primitive_type,
};

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::SerializationContract;

impl Analyzer {
    pub(super) fn validate_serialization_declarations(
        &mut self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Serialize(contract) = declaration else { continue };
            self.validate_serialization_contract(contract)?;
        }
        Ok(())
    }

    fn validate_serialization_contract(
        &mut self,
        contract: &SerializeDecl,
    ) -> Result<(), SemanticError> {
        let Some(pack) = self.pack_types.get(&contract.source_type.name) else {
            return Err(serialization_error(contract, "source type must name an existing pack"));
        };
        let Some(capacity) = pack.storage.byte_capacity() else {
            return Err(serialization_error(
                contract,
                "source pack must have byte-addressable storage",
            ));
        };
        let mut physical_ranges = validate_sections(contract, capacity)?;
        physical_ranges.sort_unstable();
        if physical_ranges.windows(2).any(|ranges| ranges[0].1 > ranges[1].0) {
            return Err(serialization_error(contract, "serialization sections overlap"));
        }
        let semantic_contract = semantic_contract(contract)?;
        self.model.serialization_contracts.push(semantic_contract);
        Ok(())
    }
}

fn semantic_contract(contract: &SerializeDecl) -> Result<SerializationContract, SemanticError> {
    let mut version = None;
    let mut payload = None;
    let mut checksum = None;
    for section in &contract.sections {
        match section {
            SerializeSection::Version { offset, .. } => version = Some(*offset),
            SerializeSection::Payload { offset, length, .. } => payload = Some((*offset, *length)),
            SerializeSection::Checksum { start, end, offset, .. } => {
                checksum = Some((*start, *end, *offset))
            }
        }
    }
    let Some(version_offset) = version else {
        return Err(serialization_error(contract, "missing version section"));
    };
    let Some((payload_offset, payload_length)) = payload else {
        return Err(serialization_error(contract, "missing payload section"));
    };
    let Some((checksum_start, checksum_end, checksum_offset)) = checksum else {
        return Err(serialization_error(contract, "missing checksum section"));
    };
    Ok(SerializationContract {
        name: contract.name.clone(),
        source_type: contract.source_type.name.clone(),
        endianness: match contract.endianness {
            crate::ast::LayoutEndianness::Little => "little".to_owned(),
            crate::ast::LayoutEndianness::Big => "big".to_owned(),
        },
        version_offset,
        payload_offset,
        payload_length,
        checksum_start,
        checksum_end,
        checksum_offset,
    })
}

fn validate_sections(
    contract: &SerializeDecl,
    capacity: u64,
) -> Result<Vec<(u16, u16)>, SemanticError> {
    let mut physical_ranges = Vec::new();
    let mut version_seen = false;
    let mut payload_seen = false;
    let mut checksum_seen = false;
    for section in &contract.sections {
        match section {
            SerializeSection::Version { ty, offset, .. } => {
                if version_seen || !is_u16(ty) {
                    return Err(serialization_error(
                        contract,
                        "version must be declared once as u16",
                    ));
                }
                version_seen = true;
                physical_ranges.push((*offset, offset.saturating_add(2)));
            }
            SerializeSection::Payload { offset, length, .. } => {
                if payload_seen || *length == 0 {
                    return Err(serialization_error(
                        contract,
                        "payload must be declared once with non-zero length",
                    ));
                }
                payload_seen = true;
                physical_ranges.push((*offset, offset.saturating_add(*length)));
            }
            SerializeSection::Checksum { start, end, offset, .. } => {
                validate_checksum(contract, capacity, *start, *end, *offset, checksum_seen)?;
                checksum_seen = true;
                physical_ranges.push((*offset, offset.saturating_add(4)));
            }
        }
    }
    if !version_seen || !payload_seen || !checksum_seen {
        return Err(serialization_error(
            contract,
            "contract requires version, payload, and checksum sections",
        ));
    }
    if physical_ranges.iter().any(|(start, end)| u64::from(*end) > capacity || start >= end) {
        return Err(serialization_error(contract, "section exceeds the source pack byte capacity"));
    }
    Ok(physical_ranges)
}

fn validate_checksum(
    contract: &SerializeDecl,
    capacity: u64,
    start: u16,
    end: u16,
    offset: u16,
    seen: bool,
) -> Result<(), SemanticError> {
    if seen || start >= end {
        return Err(serialization_error(
            contract,
            "checksum must be declared once with a non-empty range",
        ));
    }
    if u64::from(end) > capacity || u64::from(offset.saturating_add(4)) > capacity {
        return Err(serialization_error(
            contract,
            "checksum range and field must fit the source pack",
        ));
    }
    if offset < end && offset.saturating_add(4) > start {
        return Err(serialization_error(
            contract,
            "checksum field must not overlap its input range",
        ));
    }
    Ok(())
}

fn is_u16(type_name: &crate::ast::TypeName) -> bool {
    primitive_type(&type_name.name) == Some(PrimitiveType::Integer { signed: false, width: 16 })
}

fn serialization_error(contract: &SerializeDecl, reason: &str) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidSerializationContract {
            contract: contract.name.clone(),
            reason: reason.to_owned(),
        },
        span: contract.span,
    }
}

use super::contract::{BackendContractError, BackendFailure, BackendLayout};
use super::symbols::SymbolKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendEnumLayout {
    pub layout: BackendLayout,
    pub discriminant_offset: u32,
    pub discriminant_size: u8,
    pub payload_offset: u32,
    pub payload: Option<BackendLayout>,
    pub niche_pointer: bool,
}

impl BackendEnumLayout {
    pub fn validate(self) -> Result<(), BackendContractError> {
        self.layout.validate()?;
        if !matches!(self.discriminant_size, 1 | 2 | 4 | 8) {
            return Err(BackendContractError::InvalidEnumLayout(
                "discriminant size must be 1, 2, 4, or 8 bytes",
            ));
        }
        let discriminant_end = self
            .discriminant_offset
            .checked_add(u32::from(self.discriminant_size))
            .ok_or(BackendContractError::InvalidEnumLayout("discriminant range overflows"))?;
        if discriminant_end > self.layout.size {
            return Err(BackendContractError::InvalidEnumLayout(
                "discriminant range exceeds enum layout",
            ));
        }
        if let Some(payload) = self.payload {
            payload.validate()?;
            let payload_end = self
                .payload_offset
                .checked_add(payload.size)
                .ok_or(BackendContractError::InvalidEnumLayout("payload range overflows"))?;
            if payload_end > self.layout.size {
                return Err(BackendContractError::InvalidEnumLayout(
                    "payload range exceeds enum layout",
                ));
            }
            if self.payload_offset % payload.alignment != 0 {
                return Err(BackendContractError::InvalidEnumLayout(
                    "payload offset must satisfy payload alignment",
                ));
            }
        }
        if self.niche_pointer && self.payload.is_some() {
            return Err(BackendContractError::InvalidEnumLayout(
                "niche-pointer enums cannot declare an inline payload",
            ));
        }
        if self.niche_pointer
            && (self.layout.size != self.layout.alignment
                || self.discriminant_offset != 0
                || self.payload_offset != 0)
        {
            return Err(BackendContractError::InvalidEnumLayout(
                "niche-pointer enums must use a pointer-sized zero-offset representation",
            ));
        }
        Ok(())
    }
}

pub(super) fn validate_failure(failure: &BackendFailure) -> Result<(), BackendContractError> {
    if failure.result_type.kind() != SymbolKind::Enum {
        return Err(BackendContractError::InvalidFailureType);
    }
    failure.layout.validate().map_err(|_| {
        BackendContractError::InvalidEnumLayout("failure carries an invalid enum layout")
    })?;
    if failure.payload.is_some() != failure.layout.payload.is_some() {
        return Err(BackendContractError::InvalidEnumLayout(
            "failure payload must match the declared enum payload layout",
        ));
    }
    if failure.layout.niche_pointer && (failure.discriminant != 0) == failure.payload.is_none() {
        return Err(BackendContractError::InvalidEnumLayout(
            "niche-pointer failure discriminant must match its null payload state",
        ));
    }
    Ok(())
}

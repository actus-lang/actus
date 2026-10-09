use super::contract::REGION_MAX_BULK_BYTES;
use super::region::RegionError;

/// Checked logical-index, byte-offset, and bulk-copy limits for one target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionAddressProfile {
    pub logical_index_bits: u8,
    pub byte_offset_bits: u8,
    pub bulk_limit_bytes: u64,
}

impl RegionAddressProfile {
    /// Creates a profile supported by the bounded region boundary.
    pub const fn new(logical_index_bits: u8, byte_offset_bits: u8) -> Result<Self, RegionError> {
        if !supported_width(logical_index_bits) || !supported_width(byte_offset_bits) {
            return Err(RegionError::UnsupportedAddressWidth);
        }
        Ok(Self {
            logical_index_bits,
            byte_offset_bits,
            bulk_limit_bytes: REGION_MAX_BULK_BYTES as u64,
        })
    }

    /// Selects a lower bounded bulk limit for a target profile.
    pub const fn with_bulk_limit(self, bulk_limit_bytes: u64) -> Result<Self, RegionError> {
        if bulk_limit_bytes == 0 || bulk_limit_bytes > REGION_MAX_BULK_BYTES as u64 {
            return Err(RegionError::BulkLimitExceeded);
        }
        Ok(Self { bulk_limit_bytes, ..self })
    }

    pub(super) const fn validate(self) -> Result<Self, RegionError> {
        if !supported_width(self.logical_index_bits)
            || !supported_width(self.byte_offset_bits)
            || self.bulk_limit_bytes == 0
            || self.bulk_limit_bytes > REGION_MAX_BULK_BYTES as u64
        {
            return Err(RegionError::UnsupportedAddressWidth);
        }
        Ok(self)
    }

    pub(super) const fn maximum(bits: u8) -> u64 {
        if bits == 64 { u64::MAX } else { u32::MAX as u64 }
    }

    pub(super) const fn maximum_index(self) -> u64 {
        Self::maximum(self.logical_index_bits)
    }

    pub(super) const fn maximum_offset(self) -> u64 {
        Self::maximum(self.byte_offset_bits)
    }
}

pub(super) const HOST_ADDRESS_PROFILE: RegionAddressProfile = RegionAddressProfile {
    logical_index_bits: 64,
    byte_offset_bits: 64,
    bulk_limit_bytes: REGION_MAX_BULK_BYTES as u64,
};

pub(super) const fn supported_width(bits: u8) -> bool {
    bits == 32 || bits == 64
}

pub(super) const fn valid_alignment(alignment: u64, stride: u64) -> bool {
    alignment != 0 && alignment.is_power_of_two() && alignment <= stride
}

pub(super) fn copy_bulk_bytes(source: *const u8, destination: *mut u8, length: usize) {
    unsafe { std::ptr::copy(source, destination, length) }
}

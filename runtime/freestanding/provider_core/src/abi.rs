//! Stable freestanding ABI layout shared by target adapters.

use core::mem::{align_of, size_of};

use crate::{ProviderError, RegionDescriptor};

/// Buffer handle layout consumed by the generated Region bridge.
#[repr(C)]
pub struct ActusBuffer {
    /// Target-owned byte storage.
    pub data: *mut u8,
    /// Number of initialized bytes.
    pub length: usize,
    /// Number of bytes reserved by the target.
    pub capacity: usize,
}

/// Result object sizes and payload offsets emitted by the native lowering.
pub struct RegionAbiLayout;

impl RegionAbiLayout {
    /// Result discriminant offset shared by all result variants.
    pub const DISCRIMINANT_OFFSET: usize = 0usize;
    /// Payload offset for the region pointer result.
    pub const REGION_PAYLOAD_OFFSET: usize = 8usize;
    /// Payload offset for an i32 result.
    pub const INT_PAYLOAD_OFFSET: usize = 4usize;
    /// Payload offset for a u64 result.
    pub const U64_PAYLOAD_OFFSET: usize = 8usize;
    /// Result size for `Result[Int, RegionError]`.
    pub const INT_RESULT_SIZE: usize = 8usize;
    /// Result size for `Result[u64, RegionError]`.
    pub const U64_RESULT_SIZE: usize = 16usize;
    /// Result size for `Result[Region[T], RegionError]`.
    pub const REGION_RESULT_SIZE: usize = 64usize;
    /// Successful result discriminant.
    pub const OK_DISCRIMINANT: u32 = 0u32;
    /// Error result discriminant.
    pub const ERR_DISCRIMINANT: u32 = 1u32;

    /// Maps provider failures to the existing public RegionError ABI codes.
    pub const fn error_code(error: ProviderError) -> u32 {
        match error {
            ProviderError::InvalidDescriptor => 0u32,
            ProviderError::InvalidHandle => 1u32,
            ProviderError::StaleHandle => 2u32,
            ProviderError::InvalidWindow => 3u32,
            ProviderError::OffsetOverflow => 4u32,
            ProviderError::BufferSize => 5u32,
            ProviderError::CapabilityExhausted => 6u32,
            ProviderError::GenerationExhausted => 7u32,
            ProviderError::WindowLimitExceeded
            | ProviderError::InvalidAlignment
            | ProviderError::UnsupportedAddressWidth => 8u32,
        }
    }

    /// Verifies the fixed layouts before a target adapter publishes symbols.
    pub const fn validate() -> bool {
        size_of::<ActusBuffer>() >= size_of::<*mut u8>() * 3usize
            && align_of::<ActusBuffer>() == align_of::<*mut u8>()
            && size_of::<RegionDescriptor>() == 56usize
            && align_of::<RegionDescriptor>() == 8usize
            && Self::REGION_PAYLOAD_OFFSET < Self::REGION_RESULT_SIZE
            && Self::U64_PAYLOAD_OFFSET < Self::U64_RESULT_SIZE
            && Self::INT_PAYLOAD_OFFSET < Self::INT_RESULT_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_layout_matches_hosted_contract() {
        assert!(RegionAbiLayout::validate());
        assert_eq!(size_of::<ActusBuffer>(), size_of::<usize>() * 3usize);
        assert_eq!(RegionAbiLayout::error_code(ProviderError::StaleHandle), 2u32);
        assert_eq!(RegionAbiLayout::error_code(ProviderError::WindowLimitExceeded), 8u32);
    }
}

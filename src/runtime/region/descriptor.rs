use std::convert::TryFrom;

pub use super::profile::RegionAddressProfile;
use super::profile::valid_alignment;

/// Target-independent capability identifier for a runtime-backed region.
pub type RegionHandle = u64;

/// Monotonic publication generation for a runtime-backed region.
pub type RegionGeneration = u64;

/// The uninitialized generation cannot authorize a region access.
pub const UNINITIALIZED_REGION_GENERATION: RegionGeneration = 0;

/// Typed failures returned by the bounded in-memory region boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionError {
    InvalidHandle,
    StaleGeneration,
    CapabilityExhausted,
    CapabilityGenerationExhausted,
    UnsupportedAddressWidth,
    InvalidDescriptor,
    InvalidWindow,
    LogicalIndexOutOfBounds,
    OffsetOverflow,
    BufferTooSmall,
    BackendFailure,
    WindowBusy,
    GenerationExhausted,
    BulkLimitExceeded,
}

/// Fixed-width metadata for a logical region and its resident window.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionDescriptor {
    pub handle: RegionHandle,
    pub element_stride: u64,
    pub element_alignment: u64,
    pub logical_length: u64,
    pub window_start: u64,
    pub window_count: u64,
    pub generation: RegionGeneration,
    pub dirty: u8,
    pub logical_index_bits: u8,
    pub byte_offset_bits: u8,
    pub pinned: u8,
}

impl RegionDescriptor {
    /// Validates the descriptor before any native access or storage operation.
    pub fn validate(&self) -> Result<(), RegionError> {
        let profile = RegionAddressProfile::new(self.logical_index_bits, self.byte_offset_bits)?;
        if self.handle == 0
            || self.element_stride == 0
            || !valid_alignment(self.element_alignment, self.element_stride)
            || self.logical_length == 0
            || self.generation == UNINITIALIZED_REGION_GENERATION
        {
            return Err(RegionError::InvalidDescriptor);
        }
        if self.pinned > 1 {
            return Err(RegionError::InvalidDescriptor);
        }
        if self.logical_length - 1 > profile.maximum_index() {
            return Err(RegionError::OffsetOverflow);
        }
        let logical_bytes = self
            .logical_length
            .checked_mul(self.element_stride)
            .ok_or(RegionError::OffsetOverflow)?;
        if logical_bytes == 0 || logical_bytes - 1 > profile.maximum_offset() {
            return Err(RegionError::OffsetOverflow);
        }
        let window_end =
            self.window_start.checked_add(self.window_count).ok_or(RegionError::OffsetOverflow)?;
        if self.window_count == 0 || window_end > self.logical_length {
            return Err(RegionError::InvalidWindow);
        }
        Ok(())
    }

    /// Returns the stable `repr(C)` descriptor size used by the runtime ABI.
    pub const fn abi_size() -> usize {
        std::mem::size_of::<Self>()
    }

    /// Returns the stable alignment required by the runtime ABI.
    pub const fn abi_alignment() -> usize {
        std::mem::align_of::<Self>()
    }

    /// Returns the version of the fixed-width descriptor contract.
    pub const fn abi_version() -> u32 {
        super::super::contract::REGION_DESCRIPTOR_ABI_VERSION
    }

    pub(super) fn byte_range_for(
        &self,
        start_index: u64,
        element_count: u64,
        bulk_limit_bytes: u64,
    ) -> Result<std::ops::Range<usize>, RegionError> {
        let end_index =
            start_index.checked_add(element_count).ok_or(RegionError::OffsetOverflow)?;
        if end_index > self.logical_length {
            return Err(RegionError::LogicalIndexOutOfBounds);
        }
        if element_count == 0 {
            return Ok(0..0);
        }
        let window_end =
            self.window_start.checked_add(self.window_count).ok_or(RegionError::OffsetOverflow)?;
        if start_index < self.window_start || end_index > window_end {
            return Err(RegionError::LogicalIndexOutOfBounds);
        }
        let relative_index = start_index - self.window_start;
        let start =
            relative_index.checked_mul(self.element_stride).ok_or(RegionError::OffsetOverflow)?;
        let byte_count =
            element_count.checked_mul(self.element_stride).ok_or(RegionError::OffsetOverflow)?;
        if byte_count > bulk_limit_bytes {
            return Err(RegionError::BulkLimitExceeded);
        }
        let end = start.checked_add(byte_count).ok_or(RegionError::OffsetOverflow)?;
        let profile = RegionAddressProfile::new(self.logical_index_bits, self.byte_offset_bits)?;
        if end == 0 || end - 1 > profile.maximum_offset() {
            return Err(RegionError::OffsetOverflow);
        }
        let start = usize::try_from(start).map_err(|_| RegionError::OffsetOverflow)?;
        let end = usize::try_from(end).map_err(|_| RegionError::OffsetOverflow)?;
        Ok(start..end)
    }
}

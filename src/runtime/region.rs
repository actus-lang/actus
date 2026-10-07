use std::convert::TryFrom;

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
    GenerationExhausted,
}

/// Checked logical-index and byte-offset widths for one target profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionAddressProfile {
    pub logical_index_bits: u8,
    pub byte_offset_bits: u8,
}

impl RegionAddressProfile {
    /// Creates a profile supported by the bounded region boundary.
    pub const fn new(logical_index_bits: u8, byte_offset_bits: u8) -> Result<Self, RegionError> {
        if !supported_width(logical_index_bits) || !supported_width(byte_offset_bits) {
            return Err(RegionError::UnsupportedAddressWidth);
        }
        Ok(Self { logical_index_bits, byte_offset_bits })
    }

    const fn maximum(bits: u8) -> u64 {
        if bits == 64 { u64::MAX } else { u32::MAX as u64 }
    }

    const fn maximum_index(self) -> u64 {
        Self::maximum(self.logical_index_bits)
    }

    const fn maximum_offset(self) -> u64 {
        Self::maximum(self.byte_offset_bits)
    }
}

const HOST_ADDRESS_PROFILE: RegionAddressProfile =
    RegionAddressProfile { logical_index_bits: 64, byte_offset_bits: 64 };

const fn supported_width(bits: u8) -> bool {
    bits == 32 || bits == 64
}

/// Fixed-width metadata for a logical region and its resident window.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionDescriptor {
    pub handle: RegionHandle,
    pub element_stride: u64,
    pub logical_length: u64,
    pub window_start: u64,
    pub window_count: u64,
    pub generation: RegionGeneration,
    pub dirty: u8,
    pub logical_index_bits: u8,
    pub byte_offset_bits: u8,
}

impl RegionDescriptor {
    /// Validates the descriptor before any native access or storage operation.
    pub fn validate(&self) -> Result<(), RegionError> {
        let profile = RegionAddressProfile::new(self.logical_index_bits, self.byte_offset_bits)?;
        if self.handle == 0
            || self.element_stride == 0
            || self.logical_length == 0
            || self.generation == UNINITIALIZED_REGION_GENERATION
        {
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

    fn contains(&self, index: u64) -> bool {
        index >= self.window_start
            && self
                .window_start
                .checked_add(self.window_count)
                .is_some_and(|window_end| index < window_end)
    }

    fn byte_range(&self, index: u64) -> Result<std::ops::Range<usize>, RegionError> {
        if !self.contains(index) || index >= self.logical_length {
            return Err(RegionError::LogicalIndexOutOfBounds);
        }
        let relative_index = index - self.window_start;
        let start =
            relative_index.checked_mul(self.element_stride).ok_or(RegionError::OffsetOverflow)?;
        let end = start.checked_add(self.element_stride).ok_or(RegionError::OffsetOverflow)?;
        let profile = RegionAddressProfile::new(self.logical_index_bits, self.byte_offset_bits)?;
        if end == 0 || end - 1 > profile.maximum_offset() {
            return Err(RegionError::OffsetOverflow);
        }
        let start = usize::try_from(start).map_err(|_| RegionError::OffsetOverflow)?;
        let end = usize::try_from(end).map_err(|_| RegionError::OffsetOverflow)?;
        Ok(start..end)
    }
}

/// Explicit hosted backend with bounded resident storage.
pub struct InMemoryRegion {
    descriptor: RegionDescriptor,
    storage: Vec<u8>,
    published_storage: Vec<u8>,
}

impl InMemoryRegion {
    /// Creates a bounded resident window without filesystem or OS interaction.
    pub fn new(
        handle: RegionHandle,
        element_stride: u64,
        logical_length: u64,
        window_start: u64,
        window_count: u64,
    ) -> Result<Self, RegionError> {
        Self::new_with_profile(
            HOST_ADDRESS_PROFILE,
            handle,
            element_stride,
            logical_length,
            window_start,
            window_count,
        )
    }

    /// Creates a bounded resident window under an explicit target address profile.
    pub fn new_with_profile(
        profile: RegionAddressProfile,
        handle: RegionHandle,
        element_stride: u64,
        logical_length: u64,
        window_start: u64,
        window_count: u64,
    ) -> Result<Self, RegionError> {
        RegionAddressProfile::new(profile.logical_index_bits, profile.byte_offset_bits)?;
        let descriptor = RegionDescriptor {
            handle,
            element_stride,
            logical_length,
            window_start,
            window_count,
            generation: 1,
            dirty: 0,
            logical_index_bits: profile.logical_index_bits,
            byte_offset_bits: profile.byte_offset_bits,
        };
        descriptor.validate()?;
        let storage_length =
            window_count.checked_mul(element_stride).ok_or(RegionError::OffsetOverflow)?;
        let storage_length =
            usize::try_from(storage_length).map_err(|_| RegionError::OffsetOverflow)?;
        let storage = vec![0u8; storage_length];
        Ok(Self { descriptor, published_storage: storage.clone(), storage })
    }

    pub(crate) fn from_backing(
        handle: RegionHandle,
        backing: Vec<u8>,
        element_stride: u64,
        logical_length: u64,
        window_start: u64,
        window_count: u64,
    ) -> Result<Self, RegionError> {
        let expected =
            window_count.checked_mul(element_stride).ok_or(RegionError::OffsetOverflow)?;
        let expected = usize::try_from(expected).map_err(|_| RegionError::OffsetOverflow)?;
        if backing.len() != expected {
            return Err(RegionError::BufferTooSmall);
        }
        let descriptor = RegionDescriptor {
            handle,
            element_stride,
            logical_length,
            window_start,
            window_count,
            generation: 1,
            dirty: 0,
            logical_index_bits: 64,
            byte_offset_bits: 64,
        };
        descriptor.validate()?;
        Ok(Self { descriptor, published_storage: backing.clone(), storage: backing })
    }

    /// Returns the current descriptor for explicit lifecycle operations.
    pub fn descriptor(&self) -> RegionDescriptor {
        self.descriptor
    }

    /// Creates an exclusive read-only view whose lifetime cannot outlive this borrow.
    ///
    /// ```compile_fail
    /// use actus::runtime::InMemoryRegion;
    /// let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).unwrap();
    /// let generation = region.descriptor().generation;
    /// let view = region.borrow_abs(7, generation).unwrap();
    /// let _keep_view_live = &view;
    /// region.publish(generation).unwrap();
    /// let _ = view;
    /// ```
    pub fn borrow_abs(
        &mut self,
        handle: RegionHandle,
        generation: RegionGeneration,
    ) -> Result<RegionView<'_>, RegionError> {
        self.validate_access(handle, generation)?;
        Ok(RegionView { descriptor: self.descriptor, storage: &self.storage })
    }

    /// Creates a mutable view whose lifetime prevents concurrent flush or reuse.
    pub fn borrow_ins(
        &mut self,
        handle: RegionHandle,
        generation: RegionGeneration,
    ) -> Result<MutableRegionView<'_>, RegionError> {
        self.validate_access(handle, generation)?;
        Ok(MutableRegionView { descriptor: &mut self.descriptor, storage: &mut self.storage })
    }

    /// Advances the published generation after the mutable view has ended.
    pub fn publish(
        &mut self,
        generation: RegionGeneration,
    ) -> Result<RegionGeneration, RegionError> {
        self.validate_access(self.descriptor.handle, generation)?;
        let next = generation.checked_add(1).ok_or(RegionError::GenerationExhausted)?;
        self.published_storage.clone_from(&self.storage);
        self.descriptor.generation = next;
        self.descriptor.dirty = 0;
        Ok(next)
    }

    /// Discards dirty resident bytes and keeps the last published generation.
    pub fn cancel(&mut self, generation: RegionGeneration) -> Result<(), RegionError> {
        self.validate_access(self.descriptor.handle, generation)?;
        self.storage.clone_from(&self.published_storage);
        self.descriptor.dirty = 0;
        Ok(())
    }

    fn validate_access(
        &self,
        handle: RegionHandle,
        generation: RegionGeneration,
    ) -> Result<(), RegionError> {
        self.descriptor.validate()?;
        if handle != self.descriptor.handle {
            return Err(RegionError::InvalidHandle);
        }
        if generation != self.descriptor.generation {
            return Err(RegionError::StaleGeneration);
        }
        Ok(())
    }
}

/// Read-only resident view tied to the borrow that created it.
#[derive(Debug)]
pub struct RegionView<'region> {
    descriptor: RegionDescriptor,
    storage: &'region [u8],
}

impl RegionView<'_> {
    /// Copies one resident element into caller-provided storage.
    pub fn read(&self, index: u64, destination: &mut [u8]) -> Result<(), RegionError> {
        let range = self.descriptor.byte_range(index)?;
        if destination.len() != range.len() {
            return Err(RegionError::BufferTooSmall);
        }
        destination.copy_from_slice(&self.storage[range]);
        Ok(())
    }
}

/// Mutable resident view tied to the borrow that created it.
#[derive(Debug)]
pub struct MutableRegionView<'region> {
    descriptor: &'region mut RegionDescriptor,
    storage: &'region mut [u8],
}

impl MutableRegionView<'_> {
    /// Replaces one resident element and marks the descriptor dirty.
    pub fn write(&mut self, index: u64, source: &[u8]) -> Result<(), RegionError> {
        let range = self.descriptor.byte_range(index)?;
        if source.len() != range.len() {
            return Err(RegionError::BufferTooSmall);
        }
        self.storage[range].copy_from_slice(source);
        self.descriptor.dirty = 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, size_of};

    use super::{InMemoryRegion, RegionAddressProfile, RegionDescriptor, RegionError};

    #[test]
    fn bounded_views_read_write_and_publish() {
        let mut region = InMemoryRegion::new(7, 2, 8, 2, 3).expect("region should be valid");
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
            view.write(3, &[41, 42]).expect("resident element should be writable");
        }
        assert_eq!(region.descriptor().dirty, 1);
        let next = region.publish(generation).expect("publication should advance generation");
        let view = region.borrow_abs(7, next).expect("read view should open");
        let mut value = [0u8; 2];
        view.read(3, &mut value).expect("resident element should be readable");
        assert_eq!(value, [41, 42]);
    }

    #[test]
    fn stale_and_out_of_window_accesses_are_rejected() {
        let mut region = InMemoryRegion::new(7, 2, 8, 2, 3).expect("region should be valid");
        let generation = region.descriptor().generation;
        assert!(matches!(region.borrow_abs(8, generation), Err(RegionError::InvalidHandle)));
        assert_eq!(region.borrow_abs(7, generation + 1).unwrap_err(), RegionError::StaleGeneration);
        let view = region.borrow_abs(7, generation).expect("read view should open");
        let mut value = [0u8; 2];
        assert_eq!(view.read(1, &mut value), Err(RegionError::LogicalIndexOutOfBounds));
        assert_eq!(view.read(2, &mut [0u8; 1]), Err(RegionError::BufferTooSmall));
    }

    #[test]
    fn rejects_invalid_windows_and_generation_exhaustion() {
        assert!(matches!(InMemoryRegion::new(7, 2, 8, 7, 2), Err(RegionError::InvalidWindow)));
        let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
        region.descriptor.generation = u64::MAX;
        assert_eq!(region.publish(u64::MAX), Err(RegionError::GenerationExhausted));
    }

    #[test]
    fn cancellation_restores_the_last_published_window() {
        let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
            view.write(0, &[41, 42]).expect("resident element should be writable");
        }
        region.cancel(generation).expect("cancellation should succeed");
        let view = region.borrow_abs(7, generation).expect("read view should open");
        let mut value = [9u8; 2];
        view.read(0, &mut value).expect("resident element should be readable");
        assert_eq!(value, [0, 0]);
        assert_eq!(region.descriptor().dirty, 0);
    }

    #[test]
    fn failed_publication_preserves_the_current_generation() {
        let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
        let generation = region.descriptor().generation;
        assert_eq!(region.publish(generation + 1), Err(RegionError::StaleGeneration));
        assert_eq!(region.descriptor().generation, generation);
    }

    #[test]
    fn address_profiles_reject_unsupported_widths_and_large_32_bit_regions() {
        assert_eq!(RegionAddressProfile::new(16, 32), Err(RegionError::UnsupportedAddressWidth));
        let profile = RegionAddressProfile::new(32, 32).expect("32-bit profile should be valid");
        let maximum_length = u64::from(u32::MAX) + 1;
        assert!(InMemoryRegion::new_with_profile(profile, 7, 1, maximum_length, 0, 1).is_ok());
        assert!(matches!(
            InMemoryRegion::new_with_profile(profile, 7, 1, maximum_length + 1, 0, 1),
            Err(RegionError::OffsetOverflow)
        ));
        assert!(matches!(
            InMemoryRegion::new_with_profile(profile, 7, 2, maximum_length, 0, 1),
            Err(RegionError::OffsetOverflow)
        ));
    }

    #[test]
    fn wide_logical_capacity_allocates_only_the_resident_window() {
        let profile = RegionAddressProfile::new(64, 64).expect("64-bit profile should be valid");
        let logical_length = u64::from(u32::MAX) + 2;
        let mut region = InMemoryRegion::new_with_profile(profile, 7, 4, logical_length, 99, 1)
            .expect("large logical capacity should fit a small resident window");
        let generation = region.descriptor().generation;
        let view = region.borrow_abs(7, generation).expect("resident window should open");
        let mut value = [0u8; 4];
        view.read(99, &mut value).expect("resident element should be readable");
        assert_eq!(value, [0, 0, 0, 0]);
    }

    #[test]
    fn page_crossing_last_element_and_window_boundary_are_checked() {
        let mut region =
            InMemoryRegion::new(7, 2, 8194, 4095, 2).expect("cross-page window should be valid");
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
            view.write(4096, &[41, 42]).expect("last resident element should be writable");
        }
        let view = region.borrow_abs(7, generation).expect("read view should open");
        let mut value = [0u8; 2];
        view.read(4096, &mut value).expect("last resident element should be readable");
        assert_eq!(value, [41, 42]);
        assert_eq!(view.read(4097, &mut value), Err(RegionError::LogicalIndexOutOfBounds));
    }

    #[test]
    fn descriptor_uses_fixed_width_c_layout() {
        assert_eq!(size_of::<RegionDescriptor>(), RegionDescriptor::abi_size());
        assert_eq!(align_of::<RegionDescriptor>(), RegionDescriptor::abi_alignment());
        assert_eq!(RegionDescriptor::abi_size(), 56);
        assert_eq!(RegionDescriptor::abi_alignment(), 8);
    }

    #[test]
    fn invalid_descriptors_fail_through_typed_validation() {
        let valid = InMemoryRegion::new(7, 2, 8, 0, 1)
            .expect("valid region should provide a descriptor")
            .descriptor();
        let invalid_descriptors = [
            RegionDescriptor { handle: 0, ..valid },
            RegionDescriptor { element_stride: 0, ..valid },
            RegionDescriptor { logical_length: 0, ..valid },
            RegionDescriptor { window_count: 0, ..valid },
            RegionDescriptor { generation: 0, ..valid },
            RegionDescriptor { logical_index_bits: 16, ..valid },
            RegionDescriptor { logical_length: u64::MAX, ..valid },
        ];
        for descriptor in invalid_descriptors {
            assert!(descriptor.validate().is_err());
        }
    }
}

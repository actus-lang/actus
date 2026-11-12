use super::descriptor::{RegionDescriptor, RegionError, RegionGeneration, RegionHandle};
use super::profile::RegionAddressProfile;
use super::profile::{HOST_ADDRESS_PROFILE, copy_bulk_bytes};

/// Explicit hosted backend with bounded resident storage.
pub struct InMemoryRegion {
    pub(super) descriptor: RegionDescriptor,
    bulk_limit_bytes: u64,
    pub(super) storage: Vec<u8>,
    pub(super) published_storage: Vec<u8>,
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
            1,
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
        element_alignment: u64,
        logical_length: u64,
        window_start: u64,
        window_count: u64,
    ) -> Result<Self, RegionError> {
        let profile = profile.validate()?;
        let descriptor = RegionDescriptor {
            handle,
            element_stride,
            element_alignment,
            logical_length,
            window_start,
            window_count,
            generation: 1,
            dirty: 0,
            logical_index_bits: profile.logical_index_bits,
            byte_offset_bits: profile.byte_offset_bits,
            pinned: 0,
        };
        descriptor.validate()?;
        let storage_length =
            window_count.checked_mul(element_stride).ok_or(RegionError::OffsetOverflow)?;
        let storage_length =
            usize::try_from(storage_length).map_err(|_| RegionError::OffsetOverflow)?;
        let storage = vec![0u8; storage_length];
        Ok(Self {
            descriptor,
            bulk_limit_bytes: profile.bulk_limit_bytes,
            published_storage: storage.clone(),
            storage,
        })
    }

    pub(crate) fn from_backing(
        handle: RegionHandle,
        backing: Vec<u8>,
        element_stride: u64,
        element_alignment: u64,
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
            element_alignment,
            logical_length,
            window_start,
            window_count,
            generation: 1,
            dirty: 0,
            logical_index_bits: 64,
            byte_offset_bits: 64,
            pinned: 0,
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            bulk_limit_bytes: HOST_ADDRESS_PROFILE.bulk_limit_bytes,
            published_storage: backing.clone(),
            storage: backing,
        })
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
        Ok(RegionView {
            descriptor: self.descriptor,
            bulk_limit_bytes: self.bulk_limit_bytes,
            storage: &self.storage,
        })
    }

    /// Creates a mutable view whose lifetime prevents concurrent flush or reuse.
    pub fn borrow_ins(
        &mut self,
        handle: RegionHandle,
        generation: RegionGeneration,
    ) -> Result<MutableRegionView<'_>, RegionError> {
        self.validate_access(handle, generation)?;
        Ok(MutableRegionView {
            descriptor: &mut self.descriptor,
            bulk_limit_bytes: self.bulk_limit_bytes,
            storage: &mut self.storage,
        })
    }

    /// Advances the published generation after the mutable view has ended.
    pub fn publish(
        &mut self,
        generation: RegionGeneration,
    ) -> Result<RegionGeneration, RegionError> {
        self.validate_access(self.descriptor.handle, generation)?;
        let next = generation.checked_add(1).ok_or(RegionError::GenerationExhausted)?;
        self.validate_publication_storage()?;
        self.published_storage.copy_from_slice(&self.storage);
        self.descriptor.generation = next;
        self.descriptor.dirty = 0;
        Ok(next)
    }

    /// Discards dirty resident bytes and keeps the last published generation.
    pub fn cancel(&mut self, generation: RegionGeneration) -> Result<(), RegionError> {
        self.validate_access(self.descriptor.handle, generation)?;
        self.validate_publication_storage()?;
        self.storage.copy_from_slice(&self.published_storage);
        self.descriptor.dirty = 0;
        Ok(())
    }

    /// Pins the current resident window against remap and provider eviction.
    pub fn pin(&mut self, generation: RegionGeneration) -> Result<(), RegionError> {
        self.validate_access(self.descriptor.handle, generation)?;
        self.descriptor.pinned = 1;
        Ok(())
    }

    /// Releases the resident-window pin. Repeated unpin is idempotent.
    pub fn unpin(&mut self, generation: RegionGeneration) -> Result<(), RegionError> {
        self.validate_access(self.descriptor.handle, generation)?;
        self.descriptor.pinned = 0;
        Ok(())
    }

    fn validate_publication_storage(&self) -> Result<(), RegionError> {
        let expected = self
            .descriptor
            .window_count
            .checked_mul(self.descriptor.element_stride)
            .ok_or(RegionError::OffsetOverflow)?;
        let expected = usize::try_from(expected).map_err(|_| RegionError::OffsetOverflow)?;
        if self.storage.len() != expected || self.published_storage.len() != expected {
            return Err(RegionError::InvalidDescriptor);
        }
        Ok(())
    }

    /// Replaces a clean resident window and advances its generation.
    pub fn remap(
        &mut self,
        generation: RegionGeneration,
        backing: Vec<u8>,
        window_start: u64,
        window_count: u64,
    ) -> Result<RegionGeneration, RegionError> {
        self.validate_access(self.descriptor.handle, generation)?;
        if self.descriptor.dirty != 0 || self.descriptor.pinned != 0 {
            return Err(RegionError::WindowBusy);
        }
        let window_end =
            window_start.checked_add(window_count).ok_or(RegionError::OffsetOverflow)?;
        if window_count == 0 || window_end > self.descriptor.logical_length {
            return Err(RegionError::InvalidWindow);
        }
        let expected = window_count
            .checked_mul(self.descriptor.element_stride)
            .ok_or(RegionError::OffsetOverflow)?;
        let expected = usize::try_from(expected).map_err(|_| RegionError::OffsetOverflow)?;
        if backing.len() != expected {
            return Err(RegionError::BufferTooSmall);
        }
        let next = generation.checked_add(1).ok_or(RegionError::GenerationExhausted)?;
        self.storage = backing.clone();
        self.published_storage = backing;
        self.descriptor.window_start = window_start;
        self.descriptor.window_count = window_count;
        self.descriptor.generation = next;
        Ok(next)
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
    bulk_limit_bytes: u64,
    storage: &'region [u8],
}

impl RegionView<'_> {
    /// Copies one resident element into caller-provided storage.
    pub fn read(&self, index: u64, destination: &mut [u8]) -> Result<(), RegionError> {
        self.read_range(index, 1, destination).map(|_| ())
    }

    /// Copies a bounded resident range into caller-provided storage.
    pub fn read_range(
        &self,
        start_index: u64,
        element_count: u64,
        destination: &mut [u8],
    ) -> Result<u64, RegionError> {
        let range =
            self.descriptor.byte_range_for(start_index, element_count, self.bulk_limit_bytes)?;
        if destination.len() != range.len() {
            return Err(RegionError::BufferTooSmall);
        }
        copy_bulk_bytes(
            unsafe { self.storage.as_ptr().add(range.start) },
            destination.as_mut_ptr(),
            range.len(),
        );
        Ok(element_count)
    }

    /// Reads a bounded resident range with volatile source accesses.
    pub fn volatile_read_range(
        &self,
        start_index: u64,
        element_count: u64,
        destination: &mut [u8],
    ) -> Result<u64, RegionError> {
        let range =
            self.descriptor.byte_range_for(start_index, element_count, self.bulk_limit_bytes)?;
        if destination.len() != range.len() {
            return Err(RegionError::BufferTooSmall);
        }
        for (index, byte) in destination.iter_mut().enumerate() {
            *byte = unsafe { self.storage.as_ptr().add(range.start + index).read_volatile() };
        }
        Ok(element_count)
    }
}

/// Mutable resident view tied to the borrow that created it.
#[derive(Debug)]
pub struct MutableRegionView<'region> {
    descriptor: &'region mut RegionDescriptor,
    bulk_limit_bytes: u64,
    storage: &'region mut [u8],
}

impl MutableRegionView<'_> {
    /// Replaces one resident element and marks the descriptor dirty.
    pub fn write(&mut self, index: u64, source: &[u8]) -> Result<(), RegionError> {
        self.write_range(index, 1, source).map(|_| ())
    }

    /// Replaces a bounded resident range from caller-provided storage.
    pub fn write_range(
        &mut self,
        start_index: u64,
        element_count: u64,
        source: &[u8],
    ) -> Result<u64, RegionError> {
        let range =
            self.descriptor.byte_range_for(start_index, element_count, self.bulk_limit_bytes)?;
        if source.len() != range.len() {
            return Err(RegionError::BufferTooSmall);
        }
        copy_bulk_bytes(
            source.as_ptr(),
            unsafe { self.storage.as_mut_ptr().add(range.start) },
            range.len(),
        );
        self.descriptor.dirty = 1;
        Ok(element_count)
    }

    /// Writes a bounded resident range with volatile destination accesses.
    pub fn volatile_write_range(
        &mut self,
        start_index: u64,
        element_count: u64,
        source: &[u8],
    ) -> Result<u64, RegionError> {
        let range =
            self.descriptor.byte_range_for(start_index, element_count, self.bulk_limit_bytes)?;
        if source.len() != range.len() {
            return Err(RegionError::BufferTooSmall);
        }
        for (index, byte) in source.iter().copied().enumerate() {
            unsafe { self.storage.as_mut_ptr().add(range.start + index).write_volatile(byte) };
        }
        self.descriptor.dirty = 1;
        Ok(element_count)
    }
}

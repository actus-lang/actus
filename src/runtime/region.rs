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
    InvalidDescriptor,
    InvalidWindow,
    LogicalIndexOutOfBounds,
    OffsetOverflow,
    BufferTooSmall,
    GenerationExhausted,
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
}

impl RegionDescriptor {
    fn validate(&self) -> Result<(), RegionError> {
        if self.handle == 0
            || self.element_stride == 0
            || self.logical_length == 0
            || self.generation == UNINITIALIZED_REGION_GENERATION
        {
            return Err(RegionError::InvalidDescriptor);
        }
        let window_end =
            self.window_start.checked_add(self.window_count).ok_or(RegionError::OffsetOverflow)?;
        if self.window_count == 0 || window_end > self.logical_length {
            return Err(RegionError::InvalidWindow);
        }
        Ok(())
    }

    fn contains(&self, index: u64) -> bool {
        index >= self.window_start && index < self.window_start.saturating_add(self.window_count)
    }

    fn byte_range(&self, index: u64) -> Result<std::ops::Range<usize>, RegionError> {
        if !self.contains(index) || index >= self.logical_length {
            return Err(RegionError::LogicalIndexOutOfBounds);
        }
        let relative_index = index - self.window_start;
        let start =
            relative_index.checked_mul(self.element_stride).ok_or(RegionError::OffsetOverflow)?;
        let end = start.checked_add(self.element_stride).ok_or(RegionError::OffsetOverflow)?;
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
        let descriptor = RegionDescriptor {
            handle,
            element_stride,
            logical_length,
            window_start,
            window_count,
            generation: 1,
            dirty: 0,
        };
        descriptor.validate()?;
        let storage_length =
            window_count.checked_mul(element_stride).ok_or(RegionError::OffsetOverflow)?;
        let storage_length =
            usize::try_from(storage_length).map_err(|_| RegionError::OffsetOverflow)?;
        let storage = vec![0u8; storage_length];
        Ok(Self { descriptor, published_storage: storage.clone(), storage })
    }

    /// Returns the current descriptor for explicit lifecycle operations.
    pub fn descriptor(&self) -> RegionDescriptor {
        self.descriptor
    }

    /// Creates a read-only view whose lifetime cannot outlive this borrow.
    pub fn borrow_abs(
        &self,
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
    use super::{InMemoryRegion, RegionError};

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
        let region = InMemoryRegion::new(7, 2, 8, 2, 3).expect("region should be valid");
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
}

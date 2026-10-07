#![no_std]

//! Target-neutral, bounded storage provider for the freestanding Region ABI.

use core::array;

mod abi;
mod adapter;
mod pool;
#[cfg(feature = "target-abi")]
mod target_abi;
#[cfg(feature = "target-fixture")]
mod target_fixture;

pub use abi::{ActusBuffer, RegionAbiLayout};
pub use adapter::{BufferHandoffError, open_from_buffer};
pub use pool::{PoolError, StaticWindowPool, WindowLease};
#[cfg(feature = "target-abi")]
pub use target_abi::{
    actus_region_cancel, actus_region_close, actus_region_drop, actus_region_open,
    actus_region_publish, actus_region_read, actus_region_write,
};

/// Stable capability identifier: low 32 bits are slot plus one, high 32 bits are generation.
pub type RegionHandle = u64;

/// C-compatible descriptor exchanged with the generated freestanding bridge.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionDescriptor {
    /// Capability handle, never an address.
    pub handle: RegionHandle,
    /// Size of one logical element in bytes.
    pub element_stride: u64,
    /// Total logical element count.
    pub logical_length: u64,
    /// First logical element represented by the resident window.
    pub window_start: u64,
    /// Number of logical elements represented by the window.
    pub window_count: u64,
    /// Allocation generation encoded in the capability.
    pub generation: u64,
    /// Whether resident bytes differ from the published mirror.
    pub dirty: u8,
    /// Logical index width selected by the target profile.
    pub logical_index_bits: u8,
    /// Byte offset width selected by the target profile.
    pub byte_offset_bits: u8,
    /// Reserved for ABI alignment and kept zero by this core.
    pub reserved: [u8; 5],
}

/// Bounded provider failures. No allocation or operating-system error is hidden here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderError {
    /// The handle is zero, malformed, or does not identify a slot.
    InvalidHandle,
    /// The handle identifies a previously released allocation.
    StaleHandle,
    /// The descriptor does not match the active capability.
    InvalidDescriptor,
    /// A logical window is outside the declared region.
    InvalidWindow,
    /// A byte buffer is not exactly one element wide.
    BufferSize,
    /// A checked byte or element calculation overflowed.
    OffsetOverflow,
    /// No fixed capability slot is available.
    CapabilityExhausted,
    /// The requested window exceeds the target pool policy.
    WindowLimitExceeded,
    /// The target pool alignment policy is invalid or not satisfied.
    InvalidAlignment,
    /// A slot generation cannot be advanced safely.
    GenerationExhausted,
    /// The address profile is not representable by this provider.
    UnsupportedAddressWidth,
}

struct Slot<'a> {
    handle: RegionHandle,
    descriptor: RegionDescriptor,
    resident: &'a mut [u8],
    published: &'a mut [u8],
    publication_generation: u64,
}

/// Target-owned bounds for resident and published windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPolicy {
    /// Maximum bytes copied by one publish or cancel operation.
    pub max_window_bytes: u64,
    /// Required power-of-two alignment for both window bases and element stride.
    pub required_alignment: usize,
}

impl WindowPolicy {
    /// Creates an explicit bounded policy.
    pub const fn new(max_window_bytes: u64, required_alignment: usize) -> Self {
        Self { max_window_bytes, required_alignment }
    }

    /// Creates the compatibility policy used by the target-neutral core tests.
    pub const fn unbounded() -> Self {
        Self { max_window_bytes: u64::MAX, required_alignment: 1usize }
    }
}

/// Fixed-capacity provider with caller-owned resident and published windows.
pub struct Provider<'a, const SLOTS: usize> {
    slots: [Option<Slot<'a>>; SLOTS],
    generations: [u32; SLOTS],
    policy: WindowPolicy,
}

impl<'a, const SLOTS: usize> Provider<'a, SLOTS> {
    /// Creates an empty provider without allocating storage.
    pub fn new() -> Self {
        Self::with_policy(WindowPolicy::unbounded())
    }

    /// Creates a provider with a target-specific maximum window and alignment.
    pub fn with_policy(policy: WindowPolicy) -> Self {
        Self { slots: array::from_fn(|_| None), generations: [0u32; SLOTS], policy }
    }

    /// Opens one bounded resident window and its caller-provided published mirror.
    pub fn open(
        &mut self,
        resident: &'a mut [u8],
        published: &'a mut [u8],
        element_stride: u64,
        logical_length: u64,
        window_start: u64,
        window_count: u64,
        logical_index_bits: u8,
        byte_offset_bits: u8,
    ) -> Result<RegionDescriptor, ProviderError> {
        Self::validate_profile(logical_index_bits, byte_offset_bits)?;
        if element_stride == 0 || window_count == 0 {
            return Err(ProviderError::InvalidWindow);
        }
        if resident.len() != published.len() {
            return Err(ProviderError::BufferSize);
        }
        let expected_bytes =
            window_count.checked_mul(element_stride).ok_or(ProviderError::OffsetOverflow)?;
        self.validate_window_policy(resident, published, expected_bytes, element_stride)?;
        if expected_bytes != resident.len() as u64 {
            return Err(ProviderError::BufferSize);
        }
        let window_end =
            window_start.checked_add(window_count).ok_or(ProviderError::OffsetOverflow)?;
        if window_start >= logical_length || window_end > logical_length {
            return Err(ProviderError::InvalidWindow);
        }
        let slot_index = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(ProviderError::CapabilityExhausted)?;
        let generation = self.generations[slot_index]
            .checked_add(1)
            .ok_or(ProviderError::GenerationExhausted)?;
        self.generations[slot_index] = generation;
        let handle = Self::encode_handle(slot_index, generation)?;
        published.copy_from_slice(resident);
        let descriptor = RegionDescriptor {
            handle,
            element_stride,
            logical_length,
            window_start,
            window_count,
            generation: generation as u64,
            dirty: 0u8,
            logical_index_bits,
            byte_offset_bits,
            reserved: [0u8; 5],
        };
        self.slots[slot_index] =
            Some(Slot { handle, descriptor, resident, published, publication_generation: 1u64 });
        Ok(descriptor)
    }

    /// Reads one logical element from the resident window.
    pub fn read(
        &self,
        descriptor: RegionDescriptor,
        index: u64,
        destination: &mut [u8],
    ) -> Result<(), ProviderError> {
        let slot_index = self.slot_index_for(descriptor.handle)?;
        let slot = self.slots[slot_index].as_ref().ok_or(ProviderError::StaleHandle)?;
        Self::validate_descriptor(descriptor, slot)?;
        if destination.len() as u64 != descriptor.element_stride {
            return Err(ProviderError::BufferSize);
        }
        let range = Self::element_range(descriptor, index)?;
        destination.copy_from_slice(&slot.resident[range]);
        Ok(())
    }

    /// Writes one logical element into the resident window and marks it dirty.
    pub fn write(
        &mut self,
        descriptor: RegionDescriptor,
        index: u64,
        source: &[u8],
    ) -> Result<(), ProviderError> {
        let slot_index = self.slot_index_for(descriptor.handle)?;
        let slot = self.slots[slot_index].as_ref().ok_or(ProviderError::StaleHandle)?;
        Self::validate_descriptor(descriptor, slot)?;
        if source.len() as u64 != descriptor.element_stride {
            return Err(ProviderError::BufferSize);
        }
        let range = Self::element_range(descriptor, index)?;
        let slot = self.slots[slot_index].as_mut().ok_or(ProviderError::StaleHandle)?;
        slot.resident[range].copy_from_slice(source);
        slot.descriptor.dirty = 1u8;
        Ok(())
    }

    /// Publishes resident bytes into the rollback mirror.
    pub fn publish(
        &mut self,
        descriptor: RegionDescriptor,
    ) -> Result<RegionDescriptor, ProviderError> {
        let slot_index = self.slot_index_for(descriptor.handle)?;
        let slot = self.slots[slot_index].as_ref().ok_or(ProviderError::StaleHandle)?;
        Self::validate_descriptor(descriptor, slot)?;
        let slot = self.slots[slot_index].as_mut().ok_or(ProviderError::StaleHandle)?;
        slot.published.copy_from_slice(slot.resident);
        slot.publication_generation = slot
            .publication_generation
            .checked_add(1u64)
            .ok_or(ProviderError::GenerationExhausted)?;
        slot.descriptor.dirty = 0u8;
        Ok(slot.descriptor)
    }

    /// Restores the last published bytes into the resident window.
    pub fn cancel(
        &mut self,
        descriptor: RegionDescriptor,
    ) -> Result<RegionDescriptor, ProviderError> {
        let slot_index = self.slot_index_for(descriptor.handle)?;
        let slot = self.slots[slot_index].as_ref().ok_or(ProviderError::StaleHandle)?;
        Self::validate_descriptor(descriptor, slot)?;
        let slot = self.slots[slot_index].as_mut().ok_or(ProviderError::StaleHandle)?;
        slot.resident.copy_from_slice(slot.published);
        slot.descriptor.dirty = 0u8;
        Ok(slot.descriptor)
    }

    /// Closes a live capability and releases both fixed windows back to the caller.
    pub fn close(&mut self, descriptor: RegionDescriptor) -> Result<(), ProviderError> {
        let slot_index = self.slot_index_for(descriptor.handle)?;
        let slot = self.slots[slot_index].as_ref().ok_or(ProviderError::StaleHandle)?;
        Self::validate_descriptor(descriptor, slot)?;
        self.slots[slot_index] = None;
        Ok(())
    }

    /// Performs idempotent compiler-generated cleanup for a capability handle.
    pub fn drop_handle(&mut self, handle: RegionHandle) -> Result<(), ProviderError> {
        let Some(slot_index) = Self::decode_slot(handle) else {
            return Ok(());
        };
        if slot_index >= SLOTS {
            return Ok(());
        }
        if self.slots[slot_index].as_ref().is_some_and(|slot| slot.handle == handle) {
            self.slots[slot_index] = None;
        }
        Ok(())
    }

    fn slot_index_for(&self, handle: RegionHandle) -> Result<usize, ProviderError> {
        let slot_index = Self::decode_slot(handle).ok_or(ProviderError::InvalidHandle)?;
        if slot_index >= SLOTS {
            return Err(ProviderError::InvalidHandle);
        }
        let slot = self.slots[slot_index].as_ref().ok_or(ProviderError::StaleHandle)?;
        if slot.handle != handle {
            return Err(ProviderError::StaleHandle);
        }
        Ok(slot_index)
    }

    fn validate_descriptor(
        descriptor: RegionDescriptor,
        slot: &Slot<'a>,
    ) -> Result<(), ProviderError> {
        if descriptor.handle != slot.handle
            || descriptor.element_stride != slot.descriptor.element_stride
            || descriptor.logical_length != slot.descriptor.logical_length
            || descriptor.window_start != slot.descriptor.window_start
            || descriptor.window_count != slot.descriptor.window_count
            || descriptor.generation != slot.descriptor.generation
            || descriptor.logical_index_bits != slot.descriptor.logical_index_bits
            || descriptor.byte_offset_bits != slot.descriptor.byte_offset_bits
            || descriptor.reserved != [0u8; 5]
        {
            return Err(ProviderError::InvalidDescriptor);
        }
        Ok(())
    }

    fn element_range(
        descriptor: RegionDescriptor,
        index: u64,
    ) -> Result<core::ops::Range<usize>, ProviderError> {
        let window_end = descriptor
            .window_start
            .checked_add(descriptor.window_count)
            .ok_or(ProviderError::OffsetOverflow)?;
        if index < descriptor.window_start || index >= window_end {
            return Err(ProviderError::InvalidWindow);
        }
        let relative_index = index - descriptor.window_start;
        let start = relative_index
            .checked_mul(descriptor.element_stride)
            .ok_or(ProviderError::OffsetOverflow)?;
        let end =
            start.checked_add(descriptor.element_stride).ok_or(ProviderError::OffsetOverflow)?;
        let start = usize::try_from(start).map_err(|_| ProviderError::OffsetOverflow)?;
        let end = usize::try_from(end).map_err(|_| ProviderError::OffsetOverflow)?;
        Ok(start..end)
    }

    fn validate_profile(logical_index_bits: u8, byte_offset_bits: u8) -> Result<(), ProviderError> {
        if logical_index_bits > 63u8 || byte_offset_bits > 63u8 {
            return Err(ProviderError::UnsupportedAddressWidth);
        }
        Ok(())
    }

    fn validate_window_policy(
        &self,
        resident: &[u8],
        published: &[u8],
        expected_bytes: u64,
        element_stride: u64,
    ) -> Result<(), ProviderError> {
        if expected_bytes > self.policy.max_window_bytes {
            return Err(ProviderError::WindowLimitExceeded);
        }
        let alignment = self.policy.required_alignment;
        if alignment == 0usize || !alignment.is_power_of_two() {
            return Err(ProviderError::InvalidAlignment);
        }
        if element_stride % alignment as u64 != 0u64 {
            return Err(ProviderError::InvalidAlignment);
        }
        if (resident.as_ptr() as usize) % alignment != 0usize
            || (published.as_ptr() as usize) % alignment != 0usize
        {
            return Err(ProviderError::InvalidAlignment);
        }
        Ok(())
    }

    fn encode_handle(slot_index: usize, generation: u32) -> Result<RegionHandle, ProviderError> {
        let slot = u32::try_from(slot_index)
            .map_err(|_| ProviderError::CapabilityExhausted)?
            .checked_add(1u32)
            .ok_or(ProviderError::CapabilityExhausted)?;
        Ok(((generation as u64) << 32u32) | slot as u64)
    }

    fn decode_slot(handle: RegionHandle) -> Option<usize> {
        let slot = handle as u32;
        if slot == 0u32 || (handle >> 32u32) == 0u64 {
            return None;
        }
        usize::try_from(slot - 1u32).ok()
    }

    #[cfg(test)]
    fn set_generation_for_test(&mut self, slot_index: usize, generation: u32) {
        self.generations[slot_index] = generation;
    }
}

impl<'a, const SLOTS: usize> Default for Provider<'a, SLOTS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_preserves_published_bytes_for_cancel() {
        let mut resident = [1u8, 2u8, 3u8, 4u8];
        let mut published = [0u8; 4];
        let mut provider = Provider::<2>::new();
        let descriptor = provider
            .open(&mut resident, &mut published, 1u64, 4u64, 0u64, 4u64, 32u8, 32u8)
            .expect("open");
        assert_eq!(core::mem::size_of::<RegionDescriptor>(), 56usize);
        assert_eq!(core::mem::align_of::<RegionDescriptor>(), 8usize);
        assert_eq!(descriptor.logical_index_bits, 32u8);
        assert_eq!(descriptor.byte_offset_bits, 32u8);
        let mut element = [0u8; 1];
        provider.read(descriptor, 0u64, &mut element).expect("read");
        assert_eq!(element, [1u8]);
        provider.write(descriptor, 2u64, &[9u8]).expect("write");
        assert_eq!(provider.cancel(descriptor).expect("cancel").dirty, 0u8);
        provider.read(descriptor, 2u64, &mut element).expect("read");
        assert_eq!(element, [3u8]);
        provider.write(descriptor, 2u64, &[8u8]).expect("write");
        provider.publish(descriptor).expect("publish");
        provider.read(descriptor, 2u64, &mut element).expect("read");
        assert_eq!(element, [8u8]);
        provider.close(descriptor).expect("close");
        drop(provider);
        assert_eq!(resident, [1u8, 2u8, 8u8, 4u8]);
        assert_eq!(published, [1u8, 2u8, 8u8, 4u8]);
    }

    #[test]
    fn stale_generation_is_rejected_after_slot_reuse() {
        let mut first_resident = [1u8];
        let mut first_published = [0u8];
        let mut second_resident = [2u8];
        let mut second_published = [0u8];
        let mut provider = Provider::<1>::new();
        let first = provider
            .open(&mut first_resident, &mut first_published, 1u64, 1u64, 0u64, 1u64, 32u8, 32u8)
            .expect("first open");
        provider.close(first).expect("first close");
        let second = provider
            .open(&mut second_resident, &mut second_published, 1u64, 1u64, 0u64, 1u64, 32u8, 32u8)
            .expect("second open");
        assert_ne!(first.handle, second.handle);
        assert_eq!(provider.read(first, 0u64, &mut [0u8]), Err(ProviderError::StaleHandle));
    }

    #[test]
    fn capacity_and_exact_window_sizes_are_bounded() {
        let mut invalid_resident = [0u8; 2];
        let mut invalid_published = [0u8; 2];
        let mut resident = [0u8; 2];
        let mut published = [0u8; 2];
        let mut provider = Provider::<1>::new();
        assert_eq!(
            provider.open(
                &mut invalid_resident,
                &mut invalid_published,
                1u64,
                2u64,
                0u64,
                3u64,
                32u8,
                32u8
            ),
            Err(ProviderError::BufferSize)
        );
        let descriptor = provider
            .open(&mut resident, &mut published, 1u64, 2u64, 0u64, 2u64, 32u8, 32u8)
            .expect("open");
        let mut second_resident = [0u8; 1];
        let mut second_published = [0u8; 1];
        assert_eq!(
            provider.open(
                &mut second_resident,
                &mut second_published,
                1u64,
                1u64,
                0u64,
                1u64,
                32u8,
                32u8
            ),
            Err(ProviderError::CapabilityExhausted)
        );
        assert_eq!(provider.read(descriptor, 2u64, &mut [0u8]), Err(ProviderError::InvalidWindow));
    }

    #[test]
    fn drop_is_idempotent_for_live_and_stale_handles() {
        let mut resident = [7u8];
        let mut published = [0u8];
        let mut provider = Provider::<1>::new();
        let descriptor = provider
            .open(&mut resident, &mut published, 1u64, 1u64, 0u64, 1u64, 32u8, 32u8)
            .expect("open");
        assert_eq!(provider.drop_handle(descriptor.handle), Ok(()));
        assert_eq!(provider.drop_handle(descriptor.handle), Ok(()));
        assert_eq!(provider.read(descriptor, 0u64, &mut [0u8]), Err(ProviderError::StaleHandle));
    }

    #[test]
    fn window_policy_rejects_oversized_and_invalid_alignment() {
        let mut resident = [0u8; 4];
        let mut published = [0u8; 4];
        let mut bounded = Provider::<1>::with_policy(WindowPolicy::new(3u64, 1usize));
        assert_eq!(
            bounded.open(&mut resident, &mut published, 1u64, 4u64, 0u64, 4u64, 32u8, 32u8),
            Err(ProviderError::WindowLimitExceeded)
        );
        let mut invalid = Provider::<1>::with_policy(WindowPolicy::new(4u64, 3usize));
        assert_eq!(
            invalid.open(&mut resident, &mut published, 1u64, 4u64, 0u64, 4u64, 32u8, 32u8),
            Err(ProviderError::InvalidAlignment)
        );
    }

    #[test]
    fn generation_exhaustion_rejects_slot_reuse() {
        let mut resident = [0u8; 1];
        let mut published = [0u8; 1];
        let mut second_resident = [0u8; 1];
        let mut second_published = [0u8; 1];
        let mut provider = Provider::<1>::new();
        let descriptor = provider
            .open(&mut resident, &mut published, 1u64, 1u64, 0u64, 1u64, 32u8, 32u8)
            .expect("initial open");
        provider.close(descriptor).expect("close");
        provider.set_generation_for_test(0usize, u32::MAX);
        assert_eq!(
            provider.open(
                &mut second_resident,
                &mut second_published,
                1u64,
                1u64,
                0u64,
                1u64,
                32u8,
                32u8
            ),
            Err(ProviderError::GenerationExhausted)
        );
    }
}

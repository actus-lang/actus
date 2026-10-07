use std::sync::Mutex;

use super::{RegionError, RegionHandle};

/// Fixed number of hosted capability slots reserved for runtime regions.
pub const REGION_CAPABILITY_CAPACITY: usize = 1024;

const SLOT_BITS: u32 = 32;
const SLOT_MASK: u64 = u32::MAX as u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CapabilitySlot {
    generation: u32,
    active: bool,
}

impl CapabilitySlot {
    const EMPTY: Self = Self { generation: 0, active: false };
}

/// Allocation-free fixed capability table used by the hosted region bridge.
#[derive(Debug)]
pub struct RegionCapabilityTable<const CAPACITY: usize> {
    slots: [CapabilitySlot; CAPACITY],
}

impl<const CAPACITY: usize> RegionCapabilityTable<CAPACITY> {
    /// Creates an empty capability table with deterministic slot state.
    pub const fn new() -> Self {
        Self { slots: [CapabilitySlot::EMPTY; CAPACITY] }
    }

    /// Reserves one slot and returns a target-agnostic handle.
    pub fn allocate(&mut self) -> Result<RegionHandle, RegionError> {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.active {
                continue;
            }
            let generation =
                slot.generation.checked_add(1).ok_or(RegionError::CapabilityGenerationExhausted)?;
            slot.generation = generation;
            slot.active = true;
            return Ok(encode_handle(index, generation));
        }
        Err(RegionError::CapabilityExhausted)
    }

    /// Releases a live slot and rejects stale or repeated handles.
    pub fn release(&mut self, handle: RegionHandle) -> Result<(), RegionError> {
        let (index, generation) = decode_handle(handle)?;
        let slot = self.slots.get_mut(index).ok_or(RegionError::InvalidHandle)?;
        if !slot.active || slot.generation != generation {
            return Err(RegionError::InvalidHandle);
        }
        slot.active = false;
        Ok(())
    }

    /// Reports whether a handle currently names an active slot.
    pub fn contains(&self, handle: RegionHandle) -> bool {
        decode_handle(handle)
            .ok()
            .and_then(|(index, generation)| self.slots.get(index).map(|slot| (slot, generation)))
            .is_some_and(|(slot, generation)| slot.active && slot.generation == generation)
    }
}

impl<const CAPACITY: usize> Default for RegionCapabilityTable<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

static REGION_CAPABILITIES: Mutex<RegionCapabilityTable<REGION_CAPABILITY_CAPACITY>> =
    Mutex::new(RegionCapabilityTable::new());

/// Releases a native region capability during compiler-generated cleanup.
pub extern "C" fn actus_region_drop(handle: RegionHandle) -> i32 {
    let Ok(mut table) = REGION_CAPABILITIES.lock() else { return -1 };
    table.release(handle).map_or(-1, |_| 0)
}

fn encode_handle(index: usize, generation: u32) -> RegionHandle {
    (u64::from(generation) << SLOT_BITS) | (index as u64 + 1)
}

fn decode_handle(handle: RegionHandle) -> Result<(usize, u32), RegionError> {
    if handle == 0 {
        return Err(RegionError::InvalidHandle);
    }
    let raw_index = handle & SLOT_MASK;
    let generation = u32::try_from(handle >> SLOT_BITS).map_err(|_| RegionError::InvalidHandle)?;
    if raw_index == 0 || generation == 0 {
        return Err(RegionError::InvalidHandle);
    }
    Ok(((raw_index - 1) as usize, generation))
}

#[cfg(test)]
mod tests {
    use super::{RegionCapabilityTable, actus_region_drop};
    use crate::runtime::{REGION_CAPABILITY_CAPACITY, RegionError};

    #[test]
    fn capability_table_rejects_stale_and_repeated_release() {
        let mut table = RegionCapabilityTable::<2>::new();
        let handle = table.allocate().expect("first slot should allocate");
        assert!(table.contains(handle));
        table.release(handle).expect("live slot should release");
        assert!(!table.contains(handle));
        assert_eq!(table.release(handle), Err(RegionError::InvalidHandle));
    }

    #[test]
    fn capability_table_has_a_bounded_capacity() {
        let mut table = RegionCapabilityTable::<REGION_CAPABILITY_CAPACITY>::new();
        for _ in 0..REGION_CAPABILITY_CAPACITY {
            table.allocate().expect("bounded table should provide each slot");
        }
        assert_eq!(table.allocate(), Err(RegionError::CapabilityExhausted));
    }

    #[test]
    fn capability_table_rejects_generation_wraparound() {
        let mut table = RegionCapabilityTable::<1>::new();
        table.slots[0].generation = u32::MAX;
        assert_eq!(table.allocate(), Err(RegionError::CapabilityGenerationExhausted));
    }

    #[test]
    fn native_drop_bridge_rejects_unknown_handles() {
        assert_eq!(actus_region_drop(0), -1);
        assert_eq!(actus_region_drop(u64::MAX), -1);
    }
}

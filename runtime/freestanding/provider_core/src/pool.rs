//! Fixed-capacity storage pool for target adapters.

use core::array;

/// Errors returned by a statically bounded window pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolError {
    /// Every configured pool slot is already reserved.
    Exhausted,
    /// The pool cannot expose a zero-sized window.
    InvalidWindowSize,
}

/// Owned pair of equally sized, eight-byte-aligned windows.
pub struct WindowLease<const WORDS: usize> {
    slot_index: usize,
    resident: [u64; WORDS],
    published: [u64; WORDS],
}

impl<const WORDS: usize> WindowLease<WORDS> {
    /// Returns the fixed window size in bytes.
    pub const fn window_bytes() -> usize {
        WORDS * core::mem::size_of::<u64>()
    }

    /// Returns the resident and published windows as byte slices.
    ///
    /// The conversion preserves the array length and the u64 alignment. The
    /// returned borrows remain tied to this lease and cannot outlive it.
    pub fn as_slices(&mut self) -> (&mut [u8], &mut [u8]) {
        let byte_length = Self::window_bytes();
        let resident_ptr = self.resident.as_mut_ptr().cast::<u8>();
        let published_ptr = self.published.as_mut_ptr().cast::<u8>();
        // SAFETY: both arrays contain exactly WORDS initialized u64 values;
        // u64 storage is at least eight-byte aligned and the byte lengths are
        // exactly the allocated array extents.
        unsafe {
            (
                core::slice::from_raw_parts_mut(resident_ptr, byte_length),
                core::slice::from_raw_parts_mut(published_ptr, byte_length),
            )
        }
    }
}

/// Fixed-capacity pool with no allocator, filesystem, or operating-system dependency.
pub struct StaticWindowPool<const SLOTS: usize, const WORDS: usize> {
    reserved: [bool; SLOTS],
}

impl<const SLOTS: usize, const WORDS: usize> StaticWindowPool<SLOTS, WORDS> {
    /// Creates an empty pool. Each window is WORDS * 8 bytes.
    pub const fn new() -> Self {
        Self { reserved: [false; SLOTS] }
    }

    /// Reserves one resident/published window pair.
    pub fn reserve(&mut self) -> Result<WindowLease<WORDS>, PoolError> {
        if WORDS == 0usize {
            return Err(PoolError::InvalidWindowSize);
        }
        let slot_index = self
            .reserved
            .iter()
            .position(|is_reserved| !*is_reserved)
            .ok_or(PoolError::Exhausted)?;
        self.reserved[slot_index] = true;
        Ok(WindowLease {
            slot_index,
            resident: array::from_fn(|_| 0u64),
            published: array::from_fn(|_| 0u64),
        })
    }

    /// Returns a lease to the fixed pool after its provider borrow ends.
    pub fn release(&mut self, lease: WindowLease<WORDS>) {
        self.reserved[lease.slot_index] = false;
        drop(lease);
    }
}

impl<const SLOTS: usize, const WORDS: usize> Default for StaticWindowPool<SLOTS, WORDS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_is_fixed_and_windows_are_aligned() {
        let mut pool = StaticWindowPool::<1, 2>::new();
        let mut lease = pool.reserve().expect("reserve");
        assert_eq!(WindowLease::<2>::window_bytes(), 16usize);
        let (resident, published) = lease.as_slices();
        assert_eq!(resident.len(), 16usize);
        assert_eq!(published.len(), 16usize);
        assert_eq!((resident.as_ptr() as usize) % 8usize, 0usize);
        assert_eq!((published.as_ptr() as usize) % 8usize, 0usize);
        assert!(matches!(pool.reserve(), Err(PoolError::Exhausted)));
        drop((resident, published));
        pool.release(lease);
        assert!(pool.reserve().is_ok());
    }

    #[test]
    fn zero_sized_windows_are_rejected() {
        let mut pool = StaticWindowPool::<1, 0>::new();
        assert!(matches!(pool.reserve(), Err(PoolError::InvalidWindowSize)));
    }
}

//! Ownership handoff boundary between an Actus Buffer and the provider core.

use crate::{ActusBuffer, Provider, ProviderError, RegionDescriptor, WindowLease};

/// Failures before the consumed Buffer ownership can be released.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferHandoffError {
    /// The pointer or length/capacity pair is invalid.
    InvalidBuffer,
    /// The fixed lease does not exactly fit the incoming Buffer.
    BufferSize,
    /// Provider validation rejected the requested region.
    Provider(ProviderError),
}

/// Moves a validated Buffer payload into a fixed lease and opens its provider slot.
///
/// The release callback is called exactly once only after the provider accepts
/// the resident and published windows. Before that point, every failure leaves
/// the incoming Buffer untouched for its caller. The caller must keep `lease`
/// alive until the provider is dropped or closes the returned descriptor.
///
/// # Safety
///
/// `backing` must point to a live `ActusBuffer`; its data range must be readable
/// for the declared length. `release` must release that same live Buffer exactly
/// once and must not access it after this function returns successfully.
pub unsafe fn open_from_buffer<'a, const SLOTS: usize, const WORDS: usize>(
    provider: &mut Provider<'a, SLOTS>,
    lease: &'a mut WindowLease<WORDS>,
    backing: *mut ActusBuffer,
    release: unsafe extern "C" fn(*mut ActusBuffer),
    element_stride: u64,
    logical_length: u64,
    window_start: u64,
    window_count: u64,
    logical_index_bits: u8,
    byte_offset_bits: u8,
) -> Result<RegionDescriptor, BufferHandoffError> {
    let buffer = unsafe { backing.as_ref() }.ok_or(BufferHandoffError::InvalidBuffer)?;
    if buffer.length > buffer.capacity || (buffer.length > 0usize && buffer.data.is_null()) {
        return Err(BufferHandoffError::InvalidBuffer);
    }
    let (resident, published) = lease.as_slices();
    if resident.len() != buffer.length {
        return Err(BufferHandoffError::BufferSize);
    }
    let source = unsafe { core::slice::from_raw_parts(buffer.data, buffer.length) };
    resident.copy_from_slice(source);
    let descriptor = provider
        .open(
            resident,
            published,
            element_stride,
            logical_length,
            window_start,
            window_count,
            logical_index_bits,
            byte_offset_bits,
        )
        .map_err(BufferHandoffError::Provider)?;
    unsafe { release(backing) };
    Ok(descriptor)
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::StaticWindowPool;

    static RELEASE_COUNT: AtomicUsize = AtomicUsize::new(0usize);

    unsafe extern "C" fn count_release(_buffer: *mut ActusBuffer) {
        RELEASE_COUNT.fetch_add(1usize, Ordering::SeqCst);
    }

    #[test]
    fn successful_open_releases_consumed_buffer_once() {
        RELEASE_COUNT.store(0usize, Ordering::SeqCst);
        let mut storage = [1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8];
        let mut backing = ActusBuffer {
            data: storage.as_mut_ptr(),
            length: storage.len(),
            capacity: storage.len(),
        };
        let mut pool = StaticWindowPool::<1, 1>::new();
        let mut lease = pool.reserve().expect("reserve");
        let mut provider = Provider::<1>::new();
        let descriptor = unsafe {
            open_from_buffer(
                &mut provider,
                &mut lease,
                &mut backing,
                count_release,
                1u64,
                8u64,
                0u64,
                8u64,
                32u8,
                32u8,
            )
        }
        .expect("open");
        assert_ne!(descriptor.handle, 0u64);
        assert_eq!(RELEASE_COUNT.load(Ordering::SeqCst), 1usize);
        drop(provider);
        pool.release(lease);
    }

    #[test]
    fn failed_open_keeps_buffer_owned_by_caller() {
        RELEASE_COUNT.store(0usize, Ordering::SeqCst);
        let mut storage = [0u8; 8];
        let mut backing = ActusBuffer {
            data: storage.as_mut_ptr(),
            length: storage.len(),
            capacity: storage.len(),
        };
        let mut pool = StaticWindowPool::<1, 1>::new();
        let mut lease = pool.reserve().expect("reserve");
        let mut provider = Provider::<1>::new();
        let error = unsafe {
            open_from_buffer(
                &mut provider,
                &mut lease,
                &mut backing,
                count_release,
                2u64,
                8u64,
                0u64,
                8u64,
                32u8,
                32u8,
            )
        }
        .expect_err("invalid stride must fail");
        assert_eq!(error, BufferHandoffError::Provider(ProviderError::BufferSize));
        assert_eq!(RELEASE_COUNT.load(Ordering::SeqCst), 0usize);
        drop(provider);
        pool.release(lease);
        assert!(pool.reserve().is_ok());
    }
}

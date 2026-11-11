//! Hosted bridges for explicit resident-window pinning.

use super::region::RegionError;
use super::region_bridge::{REGION_STORE, error_result, result_int, validate_descriptor};

/// Pins a resident window until the matching unpin or terminal cleanup.
///
/// # Safety
///
/// `region` must point to a live runtime-owned Region descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_pin(region: *mut u8) -> *mut u8 {
    match validate_descriptor(region).and_then(|descriptor| {
        let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
        store.get_mut(descriptor.handle)?.pin(descriptor.generation)?;
        unsafe { region.add(59).write(1) };
        Ok(())
    }) {
        Ok(()) => result_int(0),
        Err(error) => error_result(8, error),
    }
}

/// Releases a resident-window pin without changing its generation.
///
/// # Safety
///
/// `region` must point to a live runtime-owned Region descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_unpin(region: *mut u8) -> *mut u8 {
    match validate_descriptor(region).and_then(|descriptor| {
        let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
        store.get_mut(descriptor.handle)?.unpin(descriptor.generation)?;
        unsafe { region.add(59).write(0) };
        Ok(())
    }) {
        Ok(()) => result_int(0),
        Err(error) => error_result(8, error),
    }
}

/// Returns one when the resident window is pinned against remap and eviction.
///
/// # Safety
///
/// `region` must point to a live runtime-owned Region descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_pinned(region: *const u8) -> *mut u8 {
    match validate_descriptor(region) {
        Ok(descriptor) => result_int(i32::from(descriptor.pinned)),
        Err(error) => error_result(8, error),
    }
}

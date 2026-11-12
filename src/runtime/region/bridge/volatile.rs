//! Hosted C-ABI bridges for explicit volatile Region memory access.

use super::super::super::types::ActusBuffer;
use super::super::descriptor::RegionError;
use super::access::{REGION_STORE, error_result, result_u64, validate_descriptor};

const RESULT_U64_SIZE: usize = 16;

fn buffer_slice<'buffer>(buffer: *const ActusBuffer) -> Result<&'buffer [u8], RegionError> {
    let buffer = unsafe { buffer.as_ref() }.ok_or(RegionError::InvalidDescriptor)?;
    if buffer.length > 0 && buffer.data.is_null() {
        return Err(RegionError::InvalidDescriptor);
    }
    Ok(unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) })
}

fn buffer_slice_mut<'buffer>(buffer: *mut ActusBuffer) -> Result<&'buffer mut [u8], RegionError> {
    let buffer = unsafe { buffer.as_mut() }.ok_or(RegionError::InvalidDescriptor)?;
    if buffer.length > 0 && buffer.data.is_null() {
        return Err(RegionError::InvalidDescriptor);
    }
    Ok(unsafe { std::slice::from_raw_parts_mut(buffer.data, buffer.length) })
}

/// Reads a bounded Region range using volatile source accesses.
///
/// # Safety
///
/// `region` and `destination` must point to live generated Actus values. The
/// destination buffer must remain valid for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_volatile_read_range(
    region: *const u8,
    start_index: u64,
    element_count: u64,
    destination: *mut ActusBuffer,
) -> *mut u8 {
    let result = validate_descriptor(region).and_then(|descriptor| {
        let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
        let region = store.get_mut(descriptor.handle)?;
        let view = region.borrow_abs(descriptor.handle, descriptor.generation)?;
        view.volatile_read_range(start_index, element_count, buffer_slice_mut(destination)?)
    });
    match result {
        Ok(count) => result_u64(count),
        Err(error) => error_result(RESULT_U64_SIZE, error),
    }
}

/// Writes a bounded Region range using volatile destination accesses.
///
/// # Safety
///
/// `region` and `source` must point to live generated Actus values. The source
/// buffer must remain valid for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_volatile_write_range(
    region: *mut u8,
    start_index: u64,
    element_count: u64,
    source: *const ActusBuffer,
) -> *mut u8 {
    let result = validate_descriptor(region).and_then(|descriptor| {
        let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
        let region = store.get_mut(descriptor.handle)?;
        let mut view = region.borrow_ins(descriptor.handle, descriptor.generation)?;
        view.volatile_write_range(start_index, element_count, buffer_slice(source)?)
    });
    match result {
        Ok(count) => result_u64(count),
        Err(error) => error_result(RESULT_U64_SIZE, error),
    }
}

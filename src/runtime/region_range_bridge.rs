//! Hosted C-ABI bridges for bounded Region range access.

use super::region::RegionError;
use super::region_bridge::{REGION_STORE, error_result, result_u64, validate_descriptor};
use super::types::ActusBuffer;

const RESULT_U64_SIZE: usize = 16;

fn buffer_slice<'buffer>(buffer: *const ActusBuffer) -> Result<&'buffer [u8], RegionError> {
    let buffer = unsafe { buffer.as_ref() }.ok_or(RegionError::InvalidHandle)?;
    if buffer.length > 0 && buffer.data.is_null() {
        return Err(RegionError::InvalidHandle);
    }
    Ok(unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) })
}

fn buffer_slice_mut<'buffer>(buffer: *mut ActusBuffer) -> Result<&'buffer mut [u8], RegionError> {
    let buffer = unsafe { buffer.as_mut() }.ok_or(RegionError::InvalidHandle)?;
    if buffer.length > 0 && buffer.data.is_null() {
        return Err(RegionError::InvalidHandle);
    }
    Ok(unsafe { std::slice::from_raw_parts_mut(buffer.data, buffer.length) })
}

fn read_range(
    descriptor: &super::region::RegionDescriptor,
    start_index: u64,
    element_count: u64,
    destination: *mut ActusBuffer,
) -> Result<u64, RegionError> {
    let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
    let region = store.get_mut(descriptor.handle)?;
    let view = region.borrow_abs(descriptor.handle, descriptor.generation)?;
    let destination = buffer_slice_mut(destination)?;
    view.read_range(start_index, element_count, destination)
}

fn write_range(
    descriptor: &mut super::region::RegionDescriptor,
    start_index: u64,
    element_count: u64,
    source: *const ActusBuffer,
) -> Result<u64, RegionError> {
    let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
    let region = store.get_mut(descriptor.handle)?;
    let source = buffer_slice(source)?;
    let mut view = region.borrow_ins(descriptor.handle, descriptor.generation)?;
    let written = view.write_range(start_index, element_count, source)?;
    descriptor.dirty = 1;
    Ok(written)
}

/// Reads a bounded resident Region range into caller-provided storage.
///
/// # Safety
///
/// `region` and `destination` must be pointers to descriptors and buffers
/// owned by the generated Actus call frame. The buffers must remain valid for
/// the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_read_range(
    region: *const u8,
    start_index: u64,
    element_count: u64,
    destination: *mut ActusBuffer,
) -> *mut u8 {
    match validate_descriptor(region)
        .and_then(|descriptor| read_range(descriptor, start_index, element_count, destination))
    {
        Ok(count) => result_u64(count),
        Err(error) => error_result(RESULT_U64_SIZE, error),
    }
}

/// Writes a bounded resident Region range from caller-provided storage.
///
/// # Safety
///
/// `region` and `source` must be pointers to descriptors and buffers owned by
/// the generated Actus call frame. The buffers must remain valid for the
/// duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_write_range(
    region: *mut u8,
    start_index: u64,
    element_count: u64,
    source: *const ActusBuffer,
) -> *mut u8 {
    if validate_descriptor(region).is_err() {
        return error_result(RESULT_U64_SIZE, RegionError::InvalidDescriptor);
    }
    let descriptor = unsafe { &mut *region.cast::<super::region::RegionDescriptor>() };
    match write_range(descriptor, start_index, element_count, source) {
        Ok(count) => result_u64(count),
        Err(error) => error_result(RESULT_U64_SIZE, error),
    }
}

//! Hosted C-ABI bridges for the standard logical-region facade.

use std::ptr;

use super::allocation::take_buffer;
use super::region::{InMemoryRegion, RegionDescriptor, RegionError, RegionHandle};
use super::types::ActusBuffer;
use super::{actus_enum_allocate, actus_enum_drop};

const RESULT_OK: u32 = 0;
const RESULT_ERR: u32 = 1;
const RESULT_INT_SIZE: usize = 8;
const RESULT_U64_SIZE: usize = 16;
const RESULT_REGION_SIZE: usize = 64;
const RESULT_REGION_PAYLOAD_OFFSET: usize = 8;
const RESULT_U64_PAYLOAD_OFFSET: usize = 8;
const RESULT_INT_PAYLOAD_OFFSET: usize = 4;
const REGION_GENERATION_OFFSET: usize = 40;
const REGION_DIRTY_OFFSET: usize = 48;

static REGION_STORE: std::sync::Mutex<RegionStore> = std::sync::Mutex::new(RegionStore::new());

struct RegionStore {
    slots: [Option<InMemoryRegion>; super::capabilities::REGION_CAPABILITY_CAPACITY],
    generations: [u32; super::capabilities::REGION_CAPABILITY_CAPACITY],
    descriptors: [Option<usize>; super::capabilities::REGION_CAPABILITY_CAPACITY],
}

impl RegionStore {
    const fn new() -> Self {
        Self {
            slots: [const { None }; super::capabilities::REGION_CAPABILITY_CAPACITY],
            generations: [0; super::capabilities::REGION_CAPABILITY_CAPACITY],
            descriptors: [None; super::capabilities::REGION_CAPABILITY_CAPACITY],
        }
    }

    fn allocate(
        &mut self,
        backing: Vec<u8>,
        element_stride: u64,
        logical_length: u64,
        window_start: u64,
        window_count: u64,
    ) -> Result<RegionDescriptor, RegionError> {
        let index = self
            .slots
            .iter()
            .enumerate()
            .find_map(|(index, slot)| {
                (slot.is_none() && self.descriptors[index].is_none()).then_some(index)
            })
            .ok_or(RegionError::CapabilityExhausted)?;
        let generation = self.generations[index]
            .checked_add(1)
            .ok_or(RegionError::CapabilityGenerationExhausted)?;
        let handle = ((u64::from(generation)) << 32) | (index as u64 + 1);
        let region = InMemoryRegion::from_backing(
            handle,
            backing,
            element_stride,
            logical_length,
            window_start,
            window_count,
        )?;
        let descriptor = region.descriptor();
        self.generations[index] = generation;
        self.slots[index] = Some(region);
        Ok(descriptor)
    }

    fn get_mut(&mut self, handle: RegionHandle) -> Result<&mut InMemoryRegion, RegionError> {
        let index = decode_store_handle(handle)?;
        let region = self.slots[index].as_mut().ok_or(RegionError::InvalidHandle)?;
        if region.descriptor().handle != handle {
            return Err(RegionError::InvalidHandle);
        }
        Ok(region)
    }

    fn attach_descriptor(
        &mut self,
        handle: RegionHandle,
        descriptor: *mut RegionDescriptor,
    ) -> Result<(), RegionError> {
        let index = decode_store_handle(handle)?;
        if self.slots[index].is_none() || self.descriptors[index].is_some() {
            return Err(RegionError::InvalidHandle);
        }
        self.descriptors[index] = Some(descriptor as usize);
        Ok(())
    }

    fn remove(&mut self, handle: RegionHandle) -> Result<Option<usize>, RegionError> {
        let index = decode_store_handle(handle)?;
        if let Some(region) = self.slots[index].take() {
            if region.descriptor().handle != handle {
                self.slots[index] = Some(region);
                return Err(RegionError::InvalidHandle);
            }
            return Ok(self.descriptors[index].take());
        }
        self.descriptors[index].take().ok_or(RegionError::InvalidHandle).map(Some)
    }

    fn close(&mut self, handle: RegionHandle, descriptor: usize) -> Result<(), RegionError> {
        let index = decode_store_handle(handle)?;
        if self.slots[index].is_none() && self.descriptors[index] == Some(descriptor) {
            return Ok(());
        }
        let region = self.slots[index].take().ok_or(RegionError::InvalidHandle)?;
        if region.descriptor().handle != handle || self.descriptors[index] != Some(descriptor) {
            self.slots[index] = Some(region);
            return Err(RegionError::InvalidHandle);
        }
        Ok(())
    }
}

fn decode_store_handle(handle: RegionHandle) -> Result<usize, RegionError> {
    if handle == 0 {
        return Err(RegionError::InvalidHandle);
    }
    let raw_index = (handle & u32::MAX as u64) as usize;
    let generation = handle >> 32;
    if raw_index == 0 || generation == 0 {
        return Err(RegionError::InvalidHandle);
    }
    let index = raw_index - 1;
    if index >= super::capabilities::REGION_CAPABILITY_CAPACITY {
        return Err(RegionError::InvalidHandle);
    }
    Ok(index)
}

fn write_u32(destination: *mut u8, offset: usize, value: u32) {
    unsafe { ptr::write_unaligned(destination.add(offset).cast::<u32>(), value) };
}

fn write_u64(destination: *mut u8, offset: usize, value: u64) {
    unsafe { ptr::write_unaligned(destination.add(offset).cast::<u64>(), value) };
}

fn allocate_result(size: usize, discriminant: u32) -> *mut u8 {
    let result = actus_enum_allocate(size);
    if result.is_null() {
        return ptr::null_mut();
    }
    write_u32(result, 0, discriminant);
    result
}

fn error_result(size: usize, error: RegionError) -> *mut u8 {
    let result = allocate_result(size, RESULT_ERR);
    if !result.is_null() {
        write_u32(result, RESULT_INT_PAYLOAD_OFFSET, error_code(error));
    }
    result
}

fn error_code(error: RegionError) -> u32 {
    match error {
        RegionError::InvalidDescriptor => 0,
        RegionError::InvalidHandle => 1,
        RegionError::StaleGeneration => 2,
        RegionError::InvalidWindow | RegionError::LogicalIndexOutOfBounds => 3,
        RegionError::OffsetOverflow => 4,
        RegionError::BufferTooSmall => 5,
        RegionError::CapabilityExhausted => 6,
        RegionError::GenerationExhausted | RegionError::CapabilityGenerationExhausted => 7,
        RegionError::BackendFailure | RegionError::UnsupportedAddressWidth => 8,
    }
}

fn validate_descriptor(region: *const u8) -> Result<&'static RegionDescriptor, RegionError> {
    if region.is_null() {
        return Err(RegionError::InvalidDescriptor);
    }
    Ok(unsafe { &*region.cast::<RegionDescriptor>() })
}

/// Releases a region descriptor and its bounded capability during cleanup.
///
/// # Safety
///
/// `handle` must be a capability value previously returned by the region
/// runtime. Invalid or repeated handles are rejected without dereferencing it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_drop(handle: RegionHandle) -> i32 {
    let Ok(mut store) = REGION_STORE.lock() else { return -1 };
    let Ok(descriptor) = store.remove(handle) else {
        return -1;
    };
    if let Some(descriptor) = descriptor {
        unsafe { drop(Box::from_raw(descriptor as *mut RegionDescriptor)) };
    }
    0
}

/// Opens a region and returns the native `Result[Region[T], RegionError]` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_open(
    backing: *mut ActusBuffer,
    element_stride: u64,
    logical_length: u64,
    window_start: u64,
    window_count: u64,
) -> *mut u8 {
    let Some(backing_bytes) = (unsafe { take_buffer(backing) }) else {
        return error_result(RESULT_REGION_SIZE, RegionError::InvalidHandle);
    };
    let mut store = match REGION_STORE.lock() {
        Ok(store) => store,
        Err(_) => return error_result(RESULT_REGION_SIZE, RegionError::BackendFailure),
    };
    match store.allocate(backing_bytes, element_stride, logical_length, window_start, window_count)
    {
        Ok(descriptor) => {
            let result = allocate_result(RESULT_REGION_SIZE, RESULT_OK);
            if result.is_null() {
                let _ = store.remove(descriptor.handle);
                return error_result(RESULT_REGION_SIZE, RegionError::BackendFailure);
            }
            let handle = descriptor.handle;
            let descriptor_pointer = Box::into_raw(Box::new(descriptor));
            if store.attach_descriptor(handle, descriptor_pointer).is_err() {
                unsafe { drop(Box::from_raw(descriptor_pointer)) };
                let _ = store.remove(descriptor.handle);
                return error_result(RESULT_REGION_SIZE, RegionError::BackendFailure);
            }
            unsafe {
                ptr::write_unaligned(
                    result.add(RESULT_REGION_PAYLOAD_OFFSET).cast::<*mut RegionDescriptor>(),
                    descriptor_pointer,
                );
            }
            result
        }
        Err(error) => error_result(RESULT_REGION_SIZE, error),
    }
}

/// Reads one resident element into the caller-provided buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_read(
    region: *const u8,
    index: u64,
    destination: *mut ActusBuffer,
) -> *mut u8 {
    match validate_descriptor(region)
        .and_then(|descriptor| read_region(descriptor, index, destination))
    {
        Ok(()) => result_int(0),
        Err(error) => error_result(RESULT_INT_SIZE, error),
    }
}

fn read_region(
    descriptor: &RegionDescriptor,
    index: u64,
    destination: *mut ActusBuffer,
) -> Result<(), RegionError> {
    let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
    let region = store.get_mut(descriptor.handle)?;
    let view = region.borrow_abs(descriptor.handle, descriptor.generation)?;
    if destination.is_null() {
        return Err(RegionError::InvalidHandle);
    }
    let destination = unsafe { &mut *destination };
    if destination.length > 0 && destination.data.is_null() {
        return Err(RegionError::InvalidHandle);
    }
    let destination_slice =
        unsafe { std::slice::from_raw_parts_mut(destination.data, destination.length) };
    view.read(index, destination_slice)
}

/// Writes one resident element from the caller-provided buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_write(
    region: *mut u8,
    index: u64,
    source: *const ActusBuffer,
) -> *mut u8 {
    if region.is_null() {
        return error_result(RESULT_INT_SIZE, RegionError::InvalidDescriptor);
    };
    match write_region(region.cast::<RegionDescriptor>(), index, source) {
        Ok(()) => result_int(0),
        Err(error) => error_result(RESULT_INT_SIZE, error),
    }
}

fn write_region(
    descriptor: *mut RegionDescriptor,
    index: u64,
    source: *const ActusBuffer,
) -> Result<(), RegionError> {
    let descriptor_ref = unsafe { &mut *descriptor };
    let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
    let region = store.get_mut(descriptor_ref.handle)?;
    let source = unsafe { source.as_ref() }.ok_or(RegionError::InvalidHandle)?;
    if source.length > 0 && source.data.is_null() {
        return Err(RegionError::InvalidHandle);
    }
    let source_slice = unsafe { std::slice::from_raw_parts(source.data, source.length) };
    let mut view = region.borrow_ins(descriptor_ref.handle, descriptor_ref.generation)?;
    view.write(index, source_slice)?;
    descriptor_ref.dirty = 1;
    Ok(())
}

/// Publishes dirty bytes and returns the next generation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_publish(region: *mut u8) -> *mut u8 {
    match validate_descriptor(region).and_then(|descriptor| {
        let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
        let backend = store.get_mut(descriptor.handle)?;
        let next = backend.publish(descriptor.generation)?;
        unsafe { ptr::write_unaligned(region.add(REGION_GENERATION_OFFSET).cast::<u64>(), next) };
        unsafe { ptr::write_unaligned(region.add(REGION_DIRTY_OFFSET), 0) };
        Ok(next)
    }) {
        Ok(next) => result_u64(next),
        Err(error) => error_result(RESULT_U64_SIZE, error),
    }
}

/// Cancels dirty bytes and keeps the current generation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_cancel(region: *mut u8) -> *mut u8 {
    match validate_descriptor(region).and_then(|descriptor| {
        let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
        store.get_mut(descriptor.handle)?.cancel(descriptor.generation)?;
        unsafe { ptr::write_unaligned(region.add(REGION_DIRTY_OFFSET), 0) };
        Ok(())
    }) {
        Ok(()) => result_int(0),
        Err(error) => error_result(RESULT_INT_SIZE, error),
    }
}

/// Closes a capability while leaving descriptor cleanup to lexical ownership teardown.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_close(region: *mut u8) -> *mut u8 {
    match validate_descriptor(region).and_then(|descriptor| {
        let mut store = REGION_STORE.lock().map_err(|_| RegionError::BackendFailure)?;
        store.close(descriptor.handle, region as usize)?;
        Ok(())
    }) {
        Ok(()) => result_int(0),
        Err(error) => error_result(RESULT_INT_SIZE, error),
    }
}

fn result_int(value: i32) -> *mut u8 {
    let result = allocate_result(RESULT_INT_SIZE, RESULT_OK);
    if !result.is_null() {
        unsafe { ptr::write_unaligned(result.add(RESULT_INT_PAYLOAD_OFFSET).cast::<i32>(), value) };
    }
    result
}

fn result_u64(value: u64) -> *mut u8 {
    let result = allocate_result(RESULT_U64_SIZE, RESULT_OK);
    if !result.is_null() {
        write_u64(result, RESULT_U64_PAYLOAD_OFFSET, value);
    }
    result
}

#[allow(dead_code)]
fn release_result(result: *mut u8, size: usize) {
    if !result.is_null() {
        unsafe { actus_enum_drop(result, size) };
    }
}

#[cfg(test)]
mod tests {
    use super::{REGION_STORE, actus_region_drop};

    #[test]
    fn native_drop_releases_a_descriptor_once() {
        let descriptor = {
            let mut store = REGION_STORE.lock().expect("region store should not be poisoned");
            let descriptor =
                store.allocate(vec![0u8; 4], 4, 1, 0, 1).expect("region slot should allocate");
            let handle = descriptor.handle;
            let pointer = Box::into_raw(Box::new(descriptor));
            store.attach_descriptor(handle, pointer).expect("descriptor should attach");
            handle
        };

        assert_eq!(unsafe { actus_region_drop(descriptor) }, 0);
        assert_eq!(unsafe { actus_region_drop(descriptor) }, -1);
    }
}

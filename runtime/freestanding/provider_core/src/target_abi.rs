//! Fixed-storage C ABI adapter for single-threaded freestanding targets.

#![allow(unsafe_op_in_unsafe_fn)]

use core::mem::MaybeUninit;
use core::ptr;

use crate::{ActusBuffer, Provider, ProviderError, RegionDescriptor};

const TARGET_SLOTS: usize = 8usize;
const TARGET_WINDOW_WORDS: usize = 128usize;
const TARGET_WINDOW_BYTES: usize = TARGET_WINDOW_WORDS * 8usize;
const RESULT_OK: u32 = 0u32;
const RESULT_ERR: u32 = 1u32;
const RESULT_INT_SIZE: usize = 8usize;
const RESULT_U64_SIZE: usize = 16usize;
const RESULT_REGION_SIZE: usize = 64usize;
const REGION_PAYLOAD_OFFSET: usize = 8usize;
const INT_PAYLOAD_OFFSET: usize = 4usize;
const U64_PAYLOAD_OFFSET: usize = 8usize;
const REGION_GENERATION_OFFSET: usize = 40usize;
const REGION_DIRTY_OFFSET: usize = 48usize;

unsafe extern "C" {
    fn actus_buffer_drop(handle: *mut ActusBuffer);
    fn actus_enum_allocate(size: usize) -> *mut u8;
}

static mut INITIALIZED: bool = false;
static mut PROVIDER: MaybeUninit<Provider<'static, TARGET_SLOTS>> = MaybeUninit::uninit();
static mut RESIDENT: [[u64; TARGET_WINDOW_WORDS]; TARGET_SLOTS] =
    [[0u64; TARGET_WINDOW_WORDS]; TARGET_SLOTS];
static mut PUBLISHED: [[u64; TARGET_WINDOW_WORDS]; TARGET_SLOTS] =
    [[0u64; TARGET_WINDOW_WORDS]; TARGET_SLOTS];
static mut USED: [bool; TARGET_SLOTS] = [false; TARGET_SLOTS];
static mut DESCRIPTORS: [MaybeUninit<RegionDescriptor>; TARGET_SLOTS] =
    [const { MaybeUninit::uninit() }; TARGET_SLOTS];

unsafe fn provider_mut() -> &'static mut Provider<'static, TARGET_SLOTS> {
    if !*ptr::addr_of!(INITIALIZED) {
        (*ptr::addr_of_mut!(PROVIDER)).write(Provider::new());
        *ptr::addr_of_mut!(INITIALIZED) = true;
    }
    &mut *(*ptr::addr_of_mut!(PROVIDER)).as_mut_ptr()
}

unsafe fn slot_bytes(slot: usize) -> (&'static mut [u8], &'static mut [u8]) {
    let resident =
        ptr::addr_of_mut!(RESIDENT).cast::<u64>().add(slot * TARGET_WINDOW_WORDS).cast::<u8>();
    let published =
        ptr::addr_of_mut!(PUBLISHED).cast::<u64>().add(slot * TARGET_WINDOW_WORDS).cast::<u8>();
    (
        core::slice::from_raw_parts_mut(resident, TARGET_WINDOW_BYTES),
        core::slice::from_raw_parts_mut(published, TARGET_WINDOW_BYTES),
    )
}

unsafe fn first_free_slot() -> Option<usize> {
    let used = ptr::read(ptr::addr_of!(USED));
    (0usize..TARGET_SLOTS).find(|slot| !used[*slot])
}

unsafe fn mark_slot(slot: usize, used: bool) {
    *ptr::addr_of_mut!(USED).cast::<bool>().add(slot) = used;
}

unsafe fn read_descriptor(region: *const u8) -> Result<RegionDescriptor, ProviderError> {
    if region.is_null() {
        return Err(ProviderError::InvalidDescriptor);
    }
    Ok(ptr::read_unaligned(region.cast::<RegionDescriptor>()))
}

unsafe fn write_descriptor(region: *mut u8, descriptor: RegionDescriptor) {
    ptr::write_unaligned(region.cast::<RegionDescriptor>(), descriptor);
}

unsafe fn allocate_result(size: usize, discriminant: u32) -> *mut u8 {
    let result = actus_enum_allocate(size);
    if !result.is_null() {
        ptr::write_unaligned(result.cast::<u32>(), discriminant);
    }
    result
}

unsafe fn error_result(size: usize, error: ProviderError) -> *mut u8 {
    let result = allocate_result(size, RESULT_ERR);
    if !result.is_null() {
        ptr::write_unaligned(
            result.add(INT_PAYLOAD_OFFSET).cast::<u32>(),
            crate::RegionAbiLayout::error_code(error),
        );
    }
    result
}

unsafe fn int_result(value: i32) -> *mut u8 {
    let result = allocate_result(RESULT_INT_SIZE, RESULT_OK);
    if !result.is_null() {
        ptr::write_unaligned(result.add(INT_PAYLOAD_OFFSET).cast::<i32>(), value);
    }
    result
}

unsafe fn u64_result(value: u64) -> *mut u8 {
    let result = allocate_result(RESULT_U64_SIZE, RESULT_OK);
    if !result.is_null() {
        ptr::write_unaligned(result.add(U64_PAYLOAD_OFFSET).cast::<u64>(), value);
    }
    result
}

/// Opens a fixed 1024-byte region window and consumes the backing Buffer on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_open(
    backing: *mut ActusBuffer,
    element_stride: u64,
    logical_length: u64,
    window_start: u64,
    window_count: u64,
) -> *mut u8 {
    let Some(buffer) = (unsafe { backing.as_ref() }) else {
        return unsafe { error_result(RESULT_REGION_SIZE, ProviderError::InvalidHandle) };
    };
    if buffer.length != TARGET_WINDOW_BYTES
        || buffer.length > buffer.capacity
        || (buffer.length > 0usize && buffer.data.is_null())
    {
        return unsafe { error_result(RESULT_REGION_SIZE, ProviderError::BufferSize) };
    }
    let Some(slot) = (unsafe { first_free_slot() }) else {
        return unsafe { error_result(RESULT_REGION_SIZE, ProviderError::CapabilityExhausted) };
    };
    let source = unsafe { core::slice::from_raw_parts(buffer.data, buffer.length) };
    let (resident, published) = unsafe { slot_bytes(slot) };
    resident.copy_from_slice(source);
    let descriptor = match unsafe { provider_mut() }.open(
        resident,
        published,
        element_stride,
        logical_length,
        window_start,
        window_count,
        32u8,
        32u8,
    ) {
        Ok(descriptor) => descriptor,
        Err(error) => return unsafe { error_result(RESULT_REGION_SIZE, error) },
    };
    unsafe {
        mark_slot(slot, true);
        (*ptr::addr_of_mut!(DESCRIPTORS).cast::<MaybeUninit<RegionDescriptor>>().add(slot))
            .write(descriptor);
        actus_buffer_drop(backing);
    }
    let result = unsafe { allocate_result(RESULT_REGION_SIZE, RESULT_OK) };
    if result.is_null() {
        unsafe {
            let _ = provider_mut().drop_handle(descriptor.handle);
            mark_slot(slot, false);
        }
        return ptr::null_mut();
    }
    unsafe {
        ptr::write_unaligned(
            result.add(REGION_PAYLOAD_OFFSET).cast::<*mut RegionDescriptor>(),
            (*ptr::addr_of_mut!(DESCRIPTORS).cast::<MaybeUninit<RegionDescriptor>>().add(slot))
                .as_mut_ptr(),
        );
    }
    result
}

/// Reads one resident element into a target-owned Buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_read(
    region: *const u8,
    index: u64,
    destination: *mut ActusBuffer,
) -> *mut u8 {
    let descriptor = match unsafe { read_descriptor(region) } {
        Ok(descriptor) => descriptor,
        Err(error) => return unsafe { error_result(RESULT_INT_SIZE, error) },
    };
    let Some(destination) = (unsafe { destination.as_mut() }) else {
        return unsafe { error_result(RESULT_INT_SIZE, ProviderError::InvalidHandle) };
    };
    if destination.length > 0usize && destination.data.is_null() {
        return unsafe { error_result(RESULT_INT_SIZE, ProviderError::InvalidHandle) };
    }
    let bytes = unsafe { core::slice::from_raw_parts_mut(destination.data, destination.length) };
    let result = unsafe { provider_mut() }.read(descriptor, index, bytes);
    match result {
        Ok(()) => unsafe { int_result(0i32) },
        Err(error) => unsafe { error_result(RESULT_INT_SIZE, error) },
    }
}

/// Writes one resident element from a target-owned Buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_write(
    region: *mut u8,
    index: u64,
    source: *const ActusBuffer,
) -> *mut u8 {
    let descriptor = match unsafe { read_descriptor(region) } {
        Ok(descriptor) => descriptor,
        Err(error) => return unsafe { error_result(RESULT_INT_SIZE, error) },
    };
    let Some(source) = (unsafe { source.as_ref() }) else {
        return unsafe { error_result(RESULT_INT_SIZE, ProviderError::InvalidHandle) };
    };
    if source.length > 0usize && source.data.is_null() {
        return unsafe { error_result(RESULT_INT_SIZE, ProviderError::InvalidHandle) };
    }
    let bytes = unsafe { core::slice::from_raw_parts(source.data, source.length) };
    match unsafe { provider_mut() }.write(descriptor, index, bytes) {
        Ok(()) => {
            unsafe { write_descriptor(region, RegionDescriptor { dirty: 1u8, ..descriptor }) };
            unsafe { int_result(0i32) }
        }
        Err(error) => unsafe { error_result(RESULT_INT_SIZE, error) },
    }
}

/// Publishes the resident window and returns its publication generation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_publish(region: *mut u8) -> *mut u8 {
    let descriptor = match unsafe { read_descriptor(region) } {
        Ok(descriptor) => descriptor,
        Err(error) => return unsafe { error_result(RESULT_U64_SIZE, error) },
    };
    match unsafe { provider_mut() }.publish(descriptor) {
        Ok(updated) => {
            unsafe { write_descriptor(region, updated) };
            unsafe { u64_result(updated.generation) }
        }
        Err(error) => unsafe { error_result(RESULT_U64_SIZE, error) },
    }
}

/// Restores the published mirror into the resident window.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_cancel(region: *mut u8) -> *mut u8 {
    let descriptor = match unsafe { read_descriptor(region) } {
        Ok(descriptor) => descriptor,
        Err(error) => return unsafe { error_result(RESULT_INT_SIZE, error) },
    };
    match unsafe { provider_mut() }.cancel(descriptor) {
        Ok(updated) => {
            unsafe { write_descriptor(region, updated) };
            unsafe { int_result(0i32) }
        }
        Err(error) => unsafe { error_result(RESULT_INT_SIZE, error) },
    }
}

/// Closes the provider capability while leaving descriptor cleanup to drop.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_close(region: *mut u8) -> *mut u8 {
    let descriptor = match unsafe { read_descriptor(region) } {
        Ok(descriptor) => descriptor,
        Err(error) => return unsafe { error_result(RESULT_INT_SIZE, error) },
    };
    match unsafe { provider_mut() }.close(descriptor) {
        Ok(()) => {
            let slot = (descriptor.handle as u32).wrapping_sub(1u32) as usize;
            if slot < TARGET_SLOTS {
                unsafe { mark_slot(slot, false) };
            }
            unsafe { int_result(0i32) }
        }
        Err(error) => unsafe { error_result(RESULT_INT_SIZE, error) },
    }
}

/// Performs idempotent cleanup for a compiler-generated region drop.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_region_drop(handle: u64) -> i32 {
    let result = unsafe { provider_mut() }.drop_handle(handle);
    let slot = (handle as u32).wrapping_sub(1u32) as usize;
    if slot < TARGET_SLOTS {
        unsafe { mark_slot(slot, false) };
    }
    if result.is_ok() { 0i32 } else { -1i32 }
}

const _: () = assert!(TARGET_WINDOW_BYTES == 1024usize);
const _: () = assert!(REGION_GENERATION_OFFSET == 40usize);
const _: () = assert!(REGION_DIRTY_OFFSET == 48usize);

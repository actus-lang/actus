#![no_std]

//! Minimal target-owned result and Buffer symbols for freestanding linkage.

use core::ptr;

/// ABI-compatible caller-owned Buffer handle consumed by region_open.
#[repr(C)]
pub struct ActusBuffer {
    pub data: *mut u8,
    pub length: usize,
    pub capacity: usize,
}

const RESULT_SLAB_BYTES: usize = 128usize;

static mut RESULT_SLAB: [u8; RESULT_SLAB_BYTES] = [0u8; RESULT_SLAB_BYTES];
static mut RESULT_BUSY: bool = false;

/// Reserves one bounded result object from target-owned storage.
#[unsafe(no_mangle)]
pub extern "C" fn actus_enum_allocate(size: usize) -> *mut u8 {
    if size == 0usize || size > RESULT_SLAB_BYTES {
        return ptr::null_mut();
    }
    unsafe {
        if *ptr::addr_of!(RESULT_BUSY) {
            return ptr::null_mut();
        }
        *ptr::addr_of_mut!(RESULT_BUSY) = true;
        ptr::addr_of_mut!(RESULT_SLAB).cast::<u8>()
    }
}

/// Releases the bounded result object when the Actus Result is dropped.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_enum_drop(pointer: *mut u8, _size: usize) {
    if pointer.is_null() {
        return;
    }
    unsafe {
        if pointer == ptr::addr_of_mut!(RESULT_SLAB).cast::<u8>() {
            *ptr::addr_of_mut!(RESULT_BUSY) = false;
        }
    }
}

/// Releases a consumed Buffer through the target-owned buffer policy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_drop(_handle: *mut ActusBuffer) {}

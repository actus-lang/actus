//! Build-only fixed target support for resolving the provider ABI object.
//!
//! This module is not a board allocator. It exists to prove that the provider
//! object can be linked against explicit no-heap target support symbols.

use core::ptr;

use crate::ActusBuffer;

const RESULT_SLAB_BYTES: usize = 128usize;
static mut RESULT_SLAB: [u8; RESULT_SLAB_BYTES] = [0u8; RESULT_SLAB_BYTES];
static mut RESULT_BUSY: bool = false;

/// Reserves the one fixed result slab used by the link fixture.
#[unsafe(no_mangle)]
pub extern "C" fn actus_enum_allocate(size: usize) -> *mut u8 {
    if size > RESULT_SLAB_BYTES || size == 0usize {
        return ptr::null_mut();
    }
    unsafe {
        if RESULT_BUSY {
            return ptr::null_mut();
        }
        RESULT_BUSY = true;
        ptr::addr_of_mut!(RESULT_SLAB).cast::<u8>()
    }
}

/// Releases the fixed result slab.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_enum_drop(pointer: *mut u8, _size: usize) {
    if pointer.is_null() {
        return;
    }
    unsafe {
        if pointer == ptr::addr_of_mut!(RESULT_SLAB).cast::<u8>() {
            RESULT_BUSY = false;
        }
    }
}

/// Target fixture release hook for a consumed Buffer handle.
///
/// A real board replaces this symbol with its fixed-pool release operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_drop(_handle: *mut ActusBuffer) {}

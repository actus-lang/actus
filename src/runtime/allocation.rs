use std::mem::ManuallyDrop;

use super::types::{ActusBuffer, BufferHandle, restore_buffer};

#[unsafe(no_mangle)]
pub extern "C" fn actus_enum_allocate(size: usize) -> *mut u8 {
    let Ok(layout) = std::alloc::Layout::from_size_align(size, 8) else {
        return std::ptr::null_mut();
    };
    unsafe { std::alloc::alloc_zeroed(layout) }
}

/// Releases storage returned by [`actus_enum_allocate`].
///
/// # Safety
///
/// `pointer` must be null or a live allocation returned by
/// [`actus_enum_allocate`] with the same `size`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_enum_drop(pointer: *mut u8, size: usize) {
    if pointer.is_null() {
        return;
    }
    let Ok(layout) = std::alloc::Layout::from_size_align(size, 8) else { return };
    unsafe { std::alloc::dealloc(pointer, layout) };
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_buffer_allocate(length: usize) -> BufferHandle {
    let mut data = Vec::<u8>::new();
    if data.try_reserve_exact(length).is_err() {
        return std::ptr::null_mut();
    }
    unsafe { data.set_len(length) };
    let data = ManuallyDrop::new(data);
    Box::into_raw(Box::new(ActusBuffer {
        data: data.as_ptr() as *mut u8,
        length: data.len(),
        capacity: data.capacity(),
    }))
}

/// Reserves reusable storage without changing the buffer length.
///
/// # Safety
///
/// `handle` must be null or a live buffer returned by
/// [`actus_buffer_allocate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_reserve(
    handle: BufferHandle,
    requested_capacity: i32,
) -> i32 {
    if handle.is_null() || requested_capacity < 0 {
        return -1;
    }
    let buffer = unsafe { &mut *handle };
    let mut data = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
    let requested = requested_capacity as usize;
    let result = if requested > data.capacity() {
        data.try_reserve_exact(requested - data.capacity()).map(|_| requested_capacity)
    } else {
        Ok(requested_capacity)
    };
    let status = result.unwrap_or(-1);
    restore_buffer(buffer, data);
    status
}

/// Releases a buffer allocated by [`actus_buffer_allocate`].
///
/// # Safety
///
/// `handle` must be null or a live handle returned by
/// [`actus_buffer_allocate`] that has not already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_drop(handle: BufferHandle) {
    if handle.is_null() {
        return;
    }
    let buffer = unsafe { Box::from_raw(handle) };
    if !buffer.data.is_null() {
        let _ = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
    }
}

/// Appends one byte to a live buffer.
///
/// # Safety
///
/// `handle` must be null or a live handle returned by
/// [`actus_buffer_allocate`] that has not already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_append(handle: BufferHandle, byte: u8) -> bool {
    if handle.is_null() {
        return false;
    }
    let buffer = unsafe { &mut *handle };
    let mut data = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
    if data.try_reserve(1).is_err() {
        restore_buffer(buffer, data);
        return false;
    }
    data.push(byte);
    restore_buffer(buffer, data);
    true
}

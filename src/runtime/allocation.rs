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
        data.try_reserve_exact(requested - data.len()).map(|_| requested_capacity)
    } else {
        Ok(requested_capacity)
    };
    let status = result.unwrap_or(-1);
    restore_buffer(buffer, data);
    status
}

/// Reserves scratch storage for the standard stream-copy operation.
///
/// # Safety
///
/// `handle` must be null or a live buffer returned by [`actus_buffer_allocate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_copy_buffer_reserve(
    handle: BufferHandle,
    requested_capacity: i32,
) -> i32 {
    unsafe { actus_buffer_reserve(handle, requested_capacity) }
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

/// Returns the reserved capacity of a live buffer, or `-1` on invalid input.
///
/// # Safety
///
/// `handle` must be null or a live buffer returned by
/// [`actus_buffer_allocate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_capacity(handle: BufferHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let buffer = unsafe { &*handle };
    i32::try_from(buffer.capacity).unwrap_or(-1)
}

/// Appends as many source bytes as fit in the target's existing capacity.
///
/// The function never reallocates. A non-negative return value is the number
/// of bytes appended; `-1` denotes invalid handles or an invalid buffer.
///
/// # Safety
///
/// Both handles must be distinct live buffers returned by
/// [`actus_buffer_allocate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_append_buffer(
    target_handle: BufferHandle,
    source_handle: BufferHandle,
) -> i32 {
    if target_handle.is_null() || source_handle.is_null() || target_handle == source_handle {
        return -1;
    }
    let target = unsafe { &mut *target_handle };
    let source = unsafe { &*source_handle };
    if target.length > target.capacity
        || (target.length > 0 && target.data.is_null())
        || (source.length > 0 && source.data.is_null())
    {
        return -1;
    }
    let available = target.capacity - target.length;
    let count = available.min(source.length);
    if count == 0 {
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(source.data, target.data.add(target.length), count);
    }
    target.length += count;
    i32::try_from(count).unwrap_or(-1)
}

/// Appends a bounded source range into existing target capacity.
///
/// The function never reallocates and returns the number of bytes appended.
/// A zero result means that the target is full; `-1` denotes invalid input.
///
/// # Safety
///
/// Both handles must be distinct live buffers returned by
/// [`actus_buffer_allocate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_append_range(
    target_handle: BufferHandle,
    source_handle: BufferHandle,
    offset: i32,
    length: i32,
) -> i32 {
    if target_handle.is_null()
        || source_handle.is_null()
        || target_handle == source_handle
        || offset < 0
        || length < 0
    {
        return -1;
    }
    let target = unsafe { &mut *target_handle };
    let source = unsafe { &*source_handle };
    let offset = offset as usize;
    let requested = length as usize;
    if target.length > target.capacity
        || offset > source.length
        || (target.length > 0 && target.data.is_null())
        || (source.length > 0 && source.data.is_null())
    {
        return -1;
    }
    let source_end = offset.saturating_add(requested).min(source.length);
    let available = target.capacity - target.length;
    let count = available.min(source_end.saturating_sub(offset));
    if count == 0 {
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(
            source.data.add(offset),
            target.data.add(target.length),
            count,
        );
    }
    target.length += count;
    i32::try_from(count).unwrap_or(-1)
}

/// Clears a buffer's logical contents while retaining its allocation.
///
/// # Safety
///
/// `handle` must be null or a live buffer returned by
/// [`actus_buffer_allocate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_clear(handle: BufferHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let buffer = unsafe { &mut *handle };
    if buffer.length > 0 && buffer.data.is_null() {
        return -1;
    }
    let count = i32::try_from(buffer.length).unwrap_or(-1);
    if count < 0 {
        return -1;
    }
    buffer.length = 0;
    count
}

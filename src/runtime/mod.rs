use std::mem::ManuallyDrop;
use std::path::Path;

pub fn runtime_archive_path() -> Option<&'static Path> {
    option_env!("ACTUS_RUNTIME_ARCHIVE").map(Path::new)
}

#[repr(C)]
pub struct ActusBuffer {
    pub data: *mut u8,
    pub length: usize,
    pub capacity: usize,
}

pub type BufferHandle = *mut ActusBuffer;

#[unsafe(no_mangle)]
pub extern "C" fn actus_buffer_allocate(length: usize) -> BufferHandle {
    let mut data = Vec::<u8>::new();
    if data.try_reserve_exact(length).is_err() {
        return std::ptr::null_mut();
    }
    unsafe { data.set_len(length) };
    let data = ManuallyDrop::new(data);
    let buffer = Box::new(ActusBuffer {
        data: data.as_ptr() as *mut u8,
        length: data.len(),
        capacity: data.capacity(),
    });
    Box::into_raw(buffer)
}

#[unsafe(no_mangle)]
/// Releases a buffer allocated by [`actus_buffer_allocate`].
///
/// # Safety
///
/// `handle` must be null or a live handle returned by
/// [`actus_buffer_allocate`] that has not already been released.
pub unsafe extern "C" fn actus_buffer_drop(handle: BufferHandle) {
    if handle.is_null() {
        return;
    }
    let buffer = unsafe { Box::from_raw(handle) };
    if !buffer.data.is_null() {
        let _ = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
    }
}

#[unsafe(no_mangle)]
/// Appends one byte to a live buffer.
///
/// # Safety
///
/// `handle` must be null or a live handle returned by
/// [`actus_buffer_allocate`] that has not already been released.
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

fn restore_buffer(buffer: &mut ActusBuffer, data: Vec<u8>) {
    let data = ManuallyDrop::new(data);
    buffer.data = data.as_ptr() as *mut u8;
    buffer.length = data.len();
    buffer.capacity = data.capacity();
}

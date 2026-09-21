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

/// Prints an integer value followed by a newline for the initial stdout API.
#[unsafe(no_mangle)]
pub extern "C" fn actus_print_int(value: i32) -> i32 {
    println!("{value}");
    value
}

#[unsafe(no_mangle)]
/// Prints a null-terminated UTF-8 string followed by a newline.
///
/// # Safety
///
/// `value` must be null or point to a valid null-terminated byte string.
pub unsafe extern "C" fn actus_print_string(value: *const u8) -> i32 {
    if value.is_null() {
        return 0;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(value.cast()) }.to_bytes();
    let text = String::from_utf8_lossy(bytes);
    println!("{text}");
    i32::try_from(bytes.len()).unwrap_or(i32::MAX)
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

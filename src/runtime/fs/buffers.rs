use std::io::Read;

use super::super::contract::ABI_STATUS_FAILURE;
use super::super::types::BufferHandle;
use super::file::{
    actus_file_close, actus_file_flush, actus_file_open, actus_file_read, actus_file_write,
    handle_to_actus, open_with_options,
};

/// Opens a file from the Actus buffer representation used by external verbs.
///
/// # Safety
/// `path` must be null or point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open_buffer(path: BufferHandle, mode: i32) -> i32 {
    if path.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let path = unsafe { &*path };
    if path.length > 0 && path.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let handle = unsafe { actus_file_open(path.data, path.length, mode as u32) };
    handle_to_actus(handle)
}

/// Opens a path from independent Actus boolean options.
///
/// # Safety
/// `path` must be null or point to a valid live `ActusBuffer` whose bytes are readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open_options_buffer(
    path: BufferHandle,
    read: i32,
    write: i32,
    append: i32,
    truncate: i32,
    create: i32,
    create_new: i32,
) -> i32 {
    if path.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let path = unsafe { &*path };
    if path.length > 0 && path.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let bytes = if path.length == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(path.data, path.length) }
    };
    let mut options = std::fs::OpenOptions::new();
    options
        .read(read != 0)
        .write(write != 0 || append != 0)
        .append(append != 0)
        .truncate(truncate != 0)
        .create(create != 0)
        .create_new(create_new != 0);
    handle_to_actus(open_with_options(bytes, options))
}

/// Reads into the spare capacity of an Actus buffer and sets its logical length.
///
/// # Safety
/// `buffer` must be null or point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_read_buffer(handle: i32, buffer: BufferHandle) -> i32 {
    if buffer.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let buffer = unsafe { &mut *buffer };
    if buffer.length > buffer.capacity || (buffer.capacity > 0 && buffer.data.is_null()) {
        return ABI_STATUS_FAILURE;
    }
    let spare = buffer.capacity.saturating_sub(buffer.length);
    let destination = if spare == 0 {
        std::ptr::NonNull::<u8>::dangling().as_ptr()
    } else {
        unsafe { buffer.data.add(buffer.length) }
    };
    let bytes = unsafe { actus_file_read(super::file::actus_handle(handle), destination, spare) };
    if bytes >= 0 {
        buffer.length = buffer.length.saturating_add(bytes as usize);
    }
    i32::try_from(bytes).unwrap_or(ABI_STATUS_FAILURE)
}

/// Reads a file to EOF into one reusable Actus buffer.
///
/// # Safety
/// `buffer` must be a valid live `ActusBuffer` with coherent length and capacity fields.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_read_all_buffer(handle: i32, buffer: BufferHandle) -> i32 {
    if buffer.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let buffer = unsafe { &mut *buffer };
    if buffer.length > buffer.capacity || (buffer.capacity > 0 && buffer.data.is_null()) {
        return ABI_STATUS_FAILURE;
    }
    let pointer = if buffer.data.is_null() {
        std::ptr::NonNull::<u8>::dangling().as_ptr()
    } else {
        buffer.data
    };
    let mut data = unsafe { Vec::from_raw_parts(pointer, buffer.length, buffer.capacity) };
    let status = super::file::with_file(super::file::actus_handle(handle), |file| {
        file.read_to_end(&mut data).map_or(ABI_STATUS_FAILURE as i64, |count| count as i64)
    });
    super::super::types::restore_buffer(buffer, data);
    i32::try_from(status).unwrap_or(ABI_STATUS_FAILURE)
}

/// Writes the logical bytes of an Actus buffer to a file.
///
/// # Safety
/// `buffer` must be null or point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_write_buffer(handle: i32, buffer: BufferHandle) -> i32 {
    if buffer.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let buffer = unsafe { &*buffer };
    if buffer.length > 0 && buffer.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let bytes =
        unsafe { actus_file_write(super::file::actus_handle(handle), buffer.data, buffer.length) };
    i32::try_from(bytes).unwrap_or(ABI_STATUS_FAILURE)
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_file_flush_buffer(handle: i32) -> i32 {
    actus_file_flush(super::file::actus_handle(handle))
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_file_close_buffer(handle: i32) -> i32 {
    actus_file_close(super::file::actus_handle(handle))
}

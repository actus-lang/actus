use std::io::Write;

use super::contract::{ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS};
use super::types::{ActusBuffer, BufferHandle};

pub(super) unsafe fn write_buffer(handle: BufferHandle, stderr: bool, newline: bool) -> i32 {
    if handle.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let buffer = unsafe { &*handle };
    if buffer.length > 0 && buffer.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    let result = if stderr {
        let mut stream = std::io::stderr().lock();
        write_bytes(&mut stream, bytes, newline)
    } else {
        let mut stream = std::io::stdout().lock();
        write_bytes(&mut stream, bytes, newline)
    };
    if result.is_err() {
        ABI_STATUS_FAILURE
    } else {
        i32::try_from(bytes.len()).unwrap_or(ABI_STATUS_FAILURE)
    }
}

/// Writes a borrowed buffer to stdout without consuming it.
///
/// # Safety
///
/// `handle` must be null or point to a valid live buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_write_buffer_stdout(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, false, false) }
}

/// Appends a borrowed source to a reusable stdout buffer.
///
/// Full buffers are written automatically. Returns accepted source bytes or
/// `-1` on validation or output failure.
///
/// # Safety
///
/// Both handles must be distinct live buffers returned by the allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffered_write_stdout(
    target_handle: BufferHandle,
    source_handle: BufferHandle,
) -> i32 {
    if target_handle.is_null() || source_handle.is_null() || target_handle == source_handle {
        return ABI_STATUS_FAILURE;
    }
    let source = unsafe { &*source_handle };
    let target = unsafe { &mut *target_handle };
    if (source.length > 0 && source.data.is_null())
        || (target.length > target.capacity)
        || (target.length > 0 && target.data.is_null())
    {
        return ABI_STATUS_FAILURE;
    }
    let source_bytes = unsafe { std::slice::from_raw_parts(source.data, source.length) };
    let mut accepted = 0_i32;
    for byte in source_bytes {
        if target.length == target.capacity
            && target.length > 0
            && unsafe { flush_raw_buffer(target) }.is_err()
        {
            return ABI_STATUS_FAILURE;
        }
        if target.length == target.capacity || target.data.is_null() {
            return ABI_STATUS_FAILURE;
        }
        unsafe { target.data.add(target.length).write(*byte) };
        target.length += 1;
        accepted = accepted.saturating_add(1);
    }
    accepted
}

/// Writes and clears pending bytes in a reusable stdout buffer.
///
/// # Safety
///
/// `handle` must be null or point to a valid live buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_flush_buffered_stdout(handle: BufferHandle) -> i32 {
    if handle.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let target = unsafe { &mut *handle };
    if target.length > target.capacity || (target.length > 0 && target.data.is_null()) {
        return ABI_STATUS_FAILURE;
    }
    let count = i32::try_from(target.length).unwrap_or(ABI_STATUS_FAILURE);
    if count < 0 || unsafe { flush_raw_buffer(target) }.is_err() {
        ABI_STATUS_FAILURE
    } else {
        count
    }
}

/// Copies an in-memory cursor source into a caller-owned destination buffer.
///
/// # Safety
///
/// Both handles must be null or live buffers returned by the allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_cursor_read(
    source_handle: BufferHandle,
    position: i32,
    target_handle: BufferHandle,
) -> i32 {
    unsafe { copy_buffer_at(source_handle, position, target_handle) }
}

/// Appends a borrowed source buffer to in-memory cursor storage.
///
/// # Safety
///
/// Both handles must be distinct live buffers returned by the allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_cursor_write(
    target_handle: BufferHandle,
    position: i32,
    source_handle: BufferHandle,
) -> i32 {
    unsafe { write_buffer_at(target_handle, position, source_handle) }
}

/// Returns the current logical length of a valid buffer, or `-1` on failure.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer returned by the allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_length(handle: BufferHandle) -> i32 {
    if handle.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let buffer = unsafe { &*handle };
    i32::try_from(buffer.length).unwrap_or(ABI_STATUS_FAILURE)
}

/// Returns zero when a buffer contains valid UTF-8, or -1 otherwise.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer returned by the allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_validate_utf8(handle: BufferHandle) -> i32 {
    if handle.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let buffer = unsafe { &*handle };
    if buffer.length == 0 {
        return ABI_STATUS_SUCCESS;
    }
    if buffer.length > 0 && buffer.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    if std::str::from_utf8(bytes).is_ok() { ABI_STATUS_SUCCESS } else { ABI_STATUS_FAILURE }
}

/// Validate an absolute cursor position against a backing buffer.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer returned by the allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_cursor_seek(handle: BufferHandle, position: i32) -> i32 {
    if handle.is_null() || position < 0 {
        return ABI_STATUS_FAILURE;
    }
    let buffer = unsafe { &*handle };
    if (position as usize) > buffer.length { ABI_STATUS_FAILURE } else { position }
}

/// Flushes cursor state. In-memory cursors have no external stream to sync.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer returned by the allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_cursor_flush(handle: BufferHandle) -> i32 {
    if handle.is_null() { ABI_STATUS_FAILURE } else { ABI_STATUS_SUCCESS }
}

unsafe fn flush_raw_buffer(buffer: &mut ActusBuffer) -> std::io::Result<()> {
    let written = unsafe { actus_write_buffer_stdout(buffer as *mut ActusBuffer) };
    if written < 0 || std::io::stdout().flush().is_err() {
        Err(std::io::Error::other("buffered stdout write failed"))
    } else {
        buffer.length = 0;
        Ok(())
    }
}

fn write_bytes(stream: &mut impl Write, bytes: &[u8], newline: bool) -> std::io::Result<()> {
    stream.write_all(bytes)?;
    if newline {
        stream.write_all(b"\n")?;
    }
    Ok(())
}

unsafe fn copy_buffer_at(
    source_handle: BufferHandle,
    position: i32,
    target_handle: BufferHandle,
) -> i32 {
    if source_handle.is_null()
        || target_handle.is_null()
        || source_handle == target_handle
        || position < 0
    {
        return ABI_STATUS_FAILURE;
    }
    let source = unsafe { &*source_handle };
    let target = unsafe { &mut *target_handle };
    if (source.length > 0 && source.data.is_null())
        || target.length > target.capacity
        || (target.length > 0 && target.data.is_null())
    {
        return ABI_STATUS_FAILURE;
    }
    let position = position as usize;
    if position > source.length {
        return ABI_STATUS_FAILURE;
    }
    let source_bytes = unsafe { std::slice::from_raw_parts(source.data, source.length) };
    let source_bytes = &source_bytes[position..];
    let mut data = unsafe { Vec::from_raw_parts(target.data, target.length, target.capacity) };
    data.clear();
    if data.try_reserve(source_bytes.len()).is_err() {
        super::types::restore_buffer(target, data);
        return ABI_STATUS_FAILURE;
    }
    data.extend_from_slice(source_bytes);
    let count = i32::try_from(source_bytes.len()).unwrap_or(ABI_STATUS_FAILURE);
    super::types::restore_buffer(target, data);
    count
}

unsafe fn write_buffer_at(
    target_handle: BufferHandle,
    position: i32,
    source_handle: BufferHandle,
) -> i32 {
    if target_handle.is_null()
        || source_handle.is_null()
        || target_handle == source_handle
        || position < 0
    {
        return ABI_STATUS_FAILURE;
    }
    let target = unsafe { &mut *target_handle };
    let source = unsafe { &*source_handle };
    if (source.length > 0 && source.data.is_null())
        || target.length > target.capacity
        || (target.length > 0 && target.data.is_null())
    {
        return ABI_STATUS_FAILURE;
    }
    let position = position as usize;
    if position > target.length {
        return ABI_STATUS_FAILURE;
    }
    let source_bytes = unsafe { std::slice::from_raw_parts(source.data, source.length) };
    let mut data = unsafe { Vec::from_raw_parts(target.data, target.length, target.capacity) };
    if data.try_reserve(source_bytes.len()).is_err() {
        super::types::restore_buffer(target, data);
        return ABI_STATUS_FAILURE;
    }
    let required = position.saturating_add(source_bytes.len());
    if required > data.len() {
        data.resize(required, 0);
    }
    data[position..required].copy_from_slice(source_bytes);
    let count = i32::try_from(source_bytes.len()).unwrap_or(ABI_STATUS_FAILURE);
    super::types::restore_buffer(target, data);
    count
}

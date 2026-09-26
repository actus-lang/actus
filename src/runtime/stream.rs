use std::io::Write;

use super::types::{ActusBuffer, BufferHandle};

pub(super) unsafe fn write_buffer(handle: BufferHandle, stderr: bool, newline: bool) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let buffer = unsafe { &*handle };
    if buffer.length > 0 && buffer.data.is_null() {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    let result = if stderr {
        let mut stream = std::io::stderr().lock();
        write_bytes(&mut stream, bytes, newline)
    } else {
        let mut stream = std::io::stdout().lock();
        write_bytes(&mut stream, bytes, newline)
    };
    if result.is_err() { -1 } else { i32::try_from(bytes.len()).unwrap_or(-1) }
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
        return -1;
    }
    let source = unsafe { &*source_handle };
    let target = unsafe { &mut *target_handle };
    if (source.length > 0 && source.data.is_null())
        || (target.length > target.capacity)
        || (target.length > 0 && target.data.is_null())
    {
        return -1;
    }
    let source_bytes = unsafe { std::slice::from_raw_parts(source.data, source.length) };
    let mut accepted = 0_i32;
    for byte in source_bytes {
        if target.length == target.capacity
            && target.length > 0
            && unsafe { flush_raw_buffer(target) }.is_err()
        {
            return -1;
        }
        if target.length == target.capacity || target.data.is_null() {
            return -1;
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
        return -1;
    }
    let target = unsafe { &mut *handle };
    if target.length > target.capacity || (target.length > 0 && target.data.is_null()) {
        return -1;
    }
    let count = i32::try_from(target.length).unwrap_or(-1);
    if count < 0 || unsafe { flush_raw_buffer(target) }.is_err() { -1 } else { count }
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

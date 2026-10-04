use std::io::Write;

use super::contract::{
    STRING_STATUS_INVALID_UTF8, STRING_STATUS_NULL, STRING_STATUS_OUT_OF_BOUNDS,
};
use super::stream::write_buffer;
use super::types::BufferHandle;

#[unsafe(no_mangle)]
pub extern "C" fn actus_print_int(value: i32) -> i32 {
    if writeln!(std::io::stdout(), "{value}").is_ok() { 0 } else { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_print_int_stderr(value: i32) -> i32 {
    if writeln!(std::io::stderr(), "{value}").is_ok() { 0 } else { -1 }
}

/// Prints a null-terminated UTF-8 string followed by a newline.
///
/// # Safety
///
/// `value` must be null or point to a valid null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_print_string(value: *const u8) -> i32 {
    if value.is_null() {
        return 0;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(value.cast()) }.to_bytes();
    println!("{}", String::from_utf8_lossy(bytes));
    i32::try_from(bytes.len()).unwrap_or(i32::MAX)
}

/// Prints a null-terminated UTF-8 string without appending a newline.
///
/// # Safety
///
/// `value` must be null or point to a valid null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_write_string_stdout(value: *const u8) -> i32 {
    if value.is_null() {
        return 0;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(value.cast()) }.to_bytes();
    if std::io::stdout().write_all(bytes).is_ok() {
        i32::try_from(bytes.len()).unwrap_or(i32::MAX)
    } else {
        -1
    }
}

/// Returns the UTF-8 byte length of a borrowed, null-terminated string.
///
/// The bridge performs validation without allocating. It returns a
/// non-negative length on success, or one of the stable string status codes
/// for null and invalid input.
///
/// # Safety
///
/// `value` must be null or point to a readable, null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_string_length(value: *const u8) -> i32 {
    let bytes = match unsafe { checked_string_bytes(value) } {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    i32::try_from(bytes.len()).unwrap_or(STRING_STATUS_OUT_OF_BOUNDS)
}

/// Returns one validated UTF-8 byte from a borrowed string without allocating.
///
/// # Safety
///
/// `value` must be null or point to a readable, null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_string_byte_at(value: *const u8, index: i32) -> i32 {
    let bytes = match unsafe { checked_string_bytes(value) } {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    if index < 0 {
        return STRING_STATUS_OUT_OF_BOUNDS;
    }
    bytes.get(index as usize).copied().map(i32::from).unwrap_or(STRING_STATUS_OUT_OF_BOUNDS)
}

unsafe fn checked_string_bytes<'a>(value: *const u8) -> Result<&'a [u8], i32> {
    if value.is_null() {
        return Err(STRING_STATUS_NULL);
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(value.cast()) }.to_bytes();
    std::str::from_utf8(bytes).map(|_| bytes).map_err(|_| STRING_STATUS_INVALID_UTF8)
}

/// Prints a null-terminated UTF-8 string to stderr followed by a newline.
///
/// # Safety
///
/// `value` must be null or point to a valid null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_print_string_stderr(value: *const u8) -> i32 {
    if value.is_null() {
        return 0;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(value.cast()) }.to_bytes();
    eprintln!("{}", String::from_utf8_lossy(bytes));
    i32::try_from(bytes.len()).unwrap_or(i32::MAX)
}

#[unsafe(no_mangle)]
///
/// # Safety
///
/// `handle` must be null or point to a valid live buffer.
pub unsafe extern "C" fn actus_print_buffer_stdout(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, false, false) }
}

#[unsafe(no_mangle)]
///
/// # Safety
///
/// `handle` must be null or point to a valid live buffer.
pub unsafe extern "C" fn actus_print_buffer_stderr(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, true, false) }
}

#[unsafe(no_mangle)]
///
/// # Safety
///
/// `handle` must be null or point to a valid live buffer.
pub unsafe extern "C" fn actus_print_line_buffer_stdout(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, false, true) }
}

#[unsafe(no_mangle)]
///
/// # Safety
///
/// `handle` must be null or point to a valid live buffer.
pub unsafe extern "C" fn actus_print_line_buffer_stderr(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, true, true) }
}

/// Flushes stdout and returns zero on success or -1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn actus_flush_stdout() -> i32 {
    if std::io::stdout().flush().is_ok() { 0 } else { -1 }
}

/// Flushes stderr and returns zero on success or -1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn actus_flush_stderr() -> i32 {
    if std::io::stderr().flush().is_ok() { 0 } else { -1 }
}

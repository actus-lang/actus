use std::io::Read;

use super::types::{BufferHandle, restore_buffer};

/// Reads bytes through the next newline into an exclusively borrowed buffer.
///
/// Returns the number of bytes read, `-2` on immediate EOF, or `-1` on error.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_read_stdin_line(handle: BufferHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let buffer = unsafe { &mut *handle };
    let mut data = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
    let mut input = std::io::stdin().lock();
    let mut byte = [0_u8; 1];
    loop {
        match input.read(&mut byte) {
            Ok(0) if data.is_empty() => {
                restore_buffer(buffer, data);
                return -2;
            }
            Ok(0) => break,
            Ok(_) if byte[0] == b'\n' => break,
            Ok(_) => data.push(byte[0]),
            Err(_) => {
                restore_buffer(buffer, data);
                return -1;
            }
        }
    }
    let count = i32::try_from(data.len()).unwrap_or(-1);
    restore_buffer(buffer, data);
    count
}

/// Reads one byte from stdin, returning `-2` at EOF or `-1` on error.
#[unsafe(no_mangle)]
pub extern "C" fn actus_read_byte() -> i32 {
    let mut byte = [0_u8; 1];
    match std::io::stdin().read(&mut byte) {
        Ok(1) => i32::from(byte[0]),
        Ok(0) => -2,
        Ok(_) | Err(_) => -1,
    }
}

use std::io::Read;
use std::io::Write;
use std::mem::ManuallyDrop;
use std::path::Path;

mod contract;

pub use contract::{
    BUFFER_ALLOCATE_SYMBOL, BUFFER_APPEND_SYMBOL, BUFFER_DROP_SYMBOL, ENUM_ALLOCATE_SYMBOL,
    ENUM_DROP_SYMBOL, FLUSH_STDOUT_SYMBOL, PRINT_BUFFER_STDERR_SYMBOL, PRINT_BUFFER_STDOUT_SYMBOL,
    PRINT_INT_STDERR_SYMBOL, PRINT_INT_SYMBOL, PRINT_LINE_BUFFER_STDERR_SYMBOL,
    PRINT_LINE_BUFFER_STDOUT_SYMBOL, PRINT_STRING_STDERR_SYMBOL, PRINT_STRING_SYMBOL,
    READ_BYTE_SYMBOL, READ_STDIN_LINE_SYMBOL, RUNTIME_ABI_VERSION, RuntimeCapability,
    WRITE_BUFFER_STDOUT_SYMBOL,
};

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

/// Prints an integer value to stderr followed by a newline.
#[unsafe(no_mangle)]
pub extern "C" fn actus_print_int_stderr(value: i32) -> i32 {
    eprintln!("{value}");
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
/// Prints a null-terminated UTF-8 string to stderr followed by a newline.
///
/// # Safety
///
/// `value` must be null or point to a valid null-terminated byte string.
pub unsafe extern "C" fn actus_print_string_stderr(value: *const u8) -> i32 {
    if value.is_null() {
        return 0;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(value.cast()) }.to_bytes();
    let text = String::from_utf8_lossy(bytes);
    eprintln!("{text}");
    i32::try_from(bytes.len()).unwrap_or(i32::MAX)
}

/// Writes the bytes in a borrowed buffer to stdout without adding a newline.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer handle whose byte range is
/// valid for the declared length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_print_buffer_stdout(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, false, false) }
}

/// Writes the bytes in a borrowed buffer to stderr without adding a newline.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer handle whose byte range is
/// valid for the declared length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_print_buffer_stderr(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, true, false) }
}

/// Writes the bytes in a borrowed buffer to stdout and adds a newline.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer handle whose byte range is
/// valid for the declared length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_print_line_buffer_stdout(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, false, true) }
}

/// Writes the bytes in a borrowed buffer to stderr and adds a newline.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer handle whose byte range is
/// valid for the declared length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_print_line_buffer_stderr(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, true, true) }
}

/// Flushes stdout and returns zero on success or -1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn actus_flush_stdout() -> i32 {
    if std::io::stdout().flush().is_ok() { 0 } else { -1 }
}

/// Reads bytes through the next newline into an exclusively borrowed buffer.
/// The newline is consumed but is not stored in the buffer.
///
/// Returns the number of bytes read, `-2` on immediate EOF, or `-1` on error.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer handle whose allocation is
/// valid for reconstruction and mutation.
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

unsafe fn write_buffer(handle: BufferHandle, stderr: bool, newline: bool) -> i32 {
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

/// Encodes borrowed stdout writes for the Actus `Result` wrapper.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer handle whose byte range is
/// valid for the declared length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_write_buffer_stdout(handle: BufferHandle) -> i32 {
    unsafe { write_buffer(handle, false, false) }
}

fn write_bytes(stream: &mut impl Write, bytes: &[u8], newline: bool) -> std::io::Result<()> {
    stream.write_all(bytes)?;
    if newline {
        stream.write_all(b"\n")?;
    }
    Ok(())
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

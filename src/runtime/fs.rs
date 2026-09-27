use std::io::{Read, Seek, SeekFrom, Write};

use super::types::BufferHandle;

mod operations;
pub use operations::{
    actus_file_copy_buffer, actus_file_create_dir_buffer, actus_file_metadata_buffer,
    actus_file_metadata_handle_buffer, actus_file_remove_buffer, actus_file_remove_dir_buffer,
    actus_file_rename_buffer,
};

const MODE_CREATE: u32 = 1;
const WHENCE_START: i32 = 0;
const WHENCE_CURRENT: i32 = 1;
const WHENCE_END: i32 = 2;

/// Opens a host file using a stable, pointer-and-length C ABI.
///
/// # Safety
/// `path` must reference `path_len` readable bytes or be null with a zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open(path: *const u8, path_len: usize, mode: u32) -> i64 {
    if path.is_null() {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(path, path_len) };
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    if mode == MODE_CREATE {
        options.write(true).create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::OsStr::from_bytes(bytes);
        return options.open(path).map_or(-1, |file| {
            use std::os::unix::io::IntoRawFd;
            file.into_raw_fd() as i64
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let wide = String::from_utf8_lossy(bytes).encode_utf16().collect::<Vec<_>>();
        let path = std::ffi::OsString::from_wide(&wide);
        use std::os::windows::io::IntoRawHandle;
        return options
            .open(path)
            .map_or(-1, |file| file.into_raw_handle() as *mut std::ffi::c_void as i64);
    }
    #[allow(unreachable_code)]
    -1
}

/// Reads into a caller-owned raw memory region.
///
/// # Safety
/// `buf` must reference `len` writable bytes or be null when `len` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_read(handle: i64, buf: *mut u8, len: usize) -> i64 {
    if handle < 0 || (len > 0 && buf.is_null()) {
        return -1;
    }
    let target =
        if len == 0 { &mut [] } else { unsafe { std::slice::from_raw_parts_mut(buf, len) } };
    with_file(handle, |file| file.read(target).map_or(-1, |count| count as i64))
}

/// Writes from a caller-owned raw memory region.
///
/// # Safety
/// `buf` must reference `len` readable bytes or be null when `len` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_write(handle: i64, buf: *const u8, len: usize) -> i64 {
    if handle < 0 || (len > 0 && buf.is_null()) {
        return -1;
    }
    let source = if len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(buf, len) } };
    with_file(handle, |file| file.write(source).map_or(-1, |count| count as i64))
}

/// Flushes a host file, returning zero or `-1`.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_flush(handle: i64) -> i32 {
    if handle < 0 {
        return -1;
    }
    with_file(handle, |file| if file.flush().is_ok() { 0 } else { -1 }) as i32
}

/// Closes a host file, returning zero or `-1`.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_close(handle: i64) -> i32 {
    if handle < 0 {
        return -1;
    }
    close_file(handle)
}

/// Moves a host file cursor and returns its resulting absolute position.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_seek(handle: i64, offset: i64, whence: i32) -> i64 {
    let origin = match whence {
        WHENCE_START if offset >= 0 => SeekFrom::Start(offset as u64),
        WHENCE_START => return -1,
        WHENCE_CURRENT => SeekFrom::Current(offset),
        WHENCE_END => SeekFrom::End(offset),
        _ => return -1,
    };
    with_file(handle, |file| file.seek(origin).map_or(-1, |position| position as i64))
}

/// Moves a file cursor through the scalar ABI used by Actus external verbs.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_seek_buffer(handle: i32, offset: i32, whence: i32) -> i32 {
    let position = actus_file_seek(i64::from(handle), i64::from(offset), whence);
    i32::try_from(position).unwrap_or(-1)
}

/// Opens a file from the Actus buffer representation used by external verbs.
///
/// # Safety
/// `path` must be null or point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open_buffer(path: BufferHandle, mode: i32) -> i32 {
    if path.is_null() {
        return -1;
    }
    let path = unsafe { &*path };
    if path.length > 0 && path.data.is_null() {
        return -1;
    }
    let handle = unsafe { actus_file_open(path.data, path.length, mode as u32) };
    handle_to_actus(handle)
}

/// Opens a path from independent Actus boolean options.
///
/// # Safety
/// `path` must be null or point to a valid live `ActusBuffer` whose bytes are
/// readable for the duration of this call.
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
        return -1;
    }
    let path = unsafe { &*path };
    if path.length > 0 && path.data.is_null() {
        return -1;
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
    let handle = open_with_options(bytes, options);
    handle_to_actus(handle)
}

/// Reads into the spare capacity of an Actus buffer and sets its logical length.
///
/// # Safety
/// `buffer` must be null or point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_read_buffer(handle: i32, buffer: BufferHandle) -> i32 {
    if buffer.is_null() {
        return -1;
    }
    let buffer = unsafe { &mut *buffer };
    if buffer.length > buffer.capacity || (buffer.capacity > 0 && buffer.data.is_null()) {
        return -1;
    }
    let spare = buffer.capacity.saturating_sub(buffer.length);
    let destination = if spare == 0 {
        std::ptr::NonNull::<u8>::dangling().as_ptr()
    } else {
        unsafe { buffer.data.add(buffer.length) }
    };
    let bytes = unsafe { actus_file_read(actus_handle(handle), destination, spare) };
    if bytes >= 0 {
        buffer.length = buffer.length.saturating_add(bytes as usize);
    }
    i32::try_from(bytes).unwrap_or(-1)
}

/// Reads a file to EOF into one reusable Actus buffer.
///
/// # Safety
/// `buffer` must be a valid live `ActusBuffer` with coherent length and
/// capacity fields.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_read_all_buffer(handle: i32, buffer: BufferHandle) -> i32 {
    if buffer.is_null() {
        return -1;
    }
    let buffer = unsafe { &mut *buffer };
    if buffer.length > buffer.capacity || (buffer.capacity > 0 && buffer.data.is_null()) {
        return -1;
    }
    let pointer = if buffer.data.is_null() {
        std::ptr::NonNull::<u8>::dangling().as_ptr()
    } else {
        buffer.data
    };
    let mut data = unsafe { Vec::from_raw_parts(pointer, buffer.length, buffer.capacity) };
    let status = with_file(actus_handle(handle), |file| {
        file.read_to_end(&mut data).map_or(-1, |count| count as i64)
    });
    super::types::restore_buffer(buffer, data);
    i32::try_from(status).unwrap_or(-1)
}

/// Writes the logical bytes of an Actus buffer to a file.
///
/// # Safety
/// `buffer` must be null or point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_write_buffer(handle: i32, buffer: BufferHandle) -> i32 {
    if buffer.is_null() {
        return -1;
    }
    let buffer = unsafe { &*buffer };
    if buffer.length > 0 && buffer.data.is_null() {
        return -1;
    }
    let bytes = unsafe { actus_file_write(actus_handle(handle), buffer.data, buffer.length) };
    i32::try_from(bytes).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_file_flush_buffer(handle: i32) -> i32 {
    actus_file_flush(actus_handle(handle))
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_file_close_buffer(handle: i32) -> i32 {
    actus_file_close(actus_handle(handle))
}

fn handle_to_actus(handle: i64) -> i32 {
    if handle < 0 {
        return -1;
    }
    #[cfg(unix)]
    {
        return i32::try_from(handle).unwrap_or(-1);
    }
    #[cfg(windows)]
    {
        return windows_handles::insert(handle);
    }
    #[allow(unreachable_code)]
    -1
}

fn open_with_options(bytes: &[u8], options: std::fs::OpenOptions) -> i64 {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::io::IntoRawFd;
        return options
            .open(std::ffi::OsStr::from_bytes(bytes))
            .map_or(-1, |file| file.into_raw_fd() as i64);
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        use std::os::windows::io::IntoRawHandle;
        let wide = String::from_utf8_lossy(bytes).encode_utf16().collect::<Vec<_>>();
        return options
            .open(std::ffi::OsString::from_wide(&wide))
            .map_or(-1, |file| file.into_raw_handle() as *mut std::ffi::c_void as i64);
    }
    #[allow(unreachable_code)]
    -1
}

pub(super) fn actus_handle(handle: i32) -> i64 {
    #[cfg(unix)]
    {
        return i64::from(handle);
    }
    #[cfg(windows)]
    {
        return windows_handles::get(handle).unwrap_or(-1);
    }
    #[allow(unreachable_code)]
    -1
}

fn close_file(handle: i64) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;
        unsafe { drop(std::fs::File::from_raw_fd(handle as i32)) };
        return 0;
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::FromRawHandle;
        unsafe { drop(std::fs::File::from_raw_handle(handle as *mut std::ffi::c_void)) };
        return 0;
    }
    #[allow(unreachable_code)]
    -1
}

pub(super) fn with_file(handle: i64, operation: impl FnOnce(&mut std::fs::File) -> i64) -> i64 {
    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;
        let mut file =
            std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(handle as i32) });
        return operation(&mut file);
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::FromRawHandle;
        let mut file = std::mem::ManuallyDrop::new(unsafe {
            std::fs::File::from_raw_handle(handle as *mut std::ffi::c_void)
        });
        return operation(&mut file);
    }
    #[allow(unreachable_code)]
    -1
}

#[cfg(windows)]
mod windows_handles {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static HANDLES: OnceLock<Mutex<HashMap<i32, i64>>> = OnceLock::new();
    static NEXT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

    fn table() -> &'static Mutex<HashMap<i32, i64>> {
        HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
    }
    pub fn insert(handle: i64) -> i32 {
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        table().lock().expect("handle table poisoned").insert(id, handle);
        id
    }
    pub fn get(id: i32) -> Option<i64> {
        table().lock().ok()?.get(&id).copied()
    }
}

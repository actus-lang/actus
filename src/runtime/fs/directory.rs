use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::ReadDir;
use std::sync::{Mutex, OnceLock};

use super::super::contract::{ABI_HANDLE_FAILURE, ABI_STATUS_END_OF_STREAM, ABI_STATUS_FAILURE};
use super::super::path::ActusPath;
use super::super::types::{ActusBuffer, BufferHandle};
use super::path_bridge::with_host_path;

const DIRECTORY_STATUS_BUFFER_TOO_SMALL: i32 = -3;

static DIRECTORIES: OnceLock<Mutex<HashMap<i32, ReadDir>>> = OnceLock::new();
static NEXT_DIRECTORY: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

fn directories() -> &'static Mutex<HashMap<i32, ReadDir>> {
    DIRECTORIES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Opens one host directory and returns an opaque runtime token.
///
/// # Safety
/// `path` must be a valid immutable `ActusPath` for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_directory_open(path: *const ActusPath) -> i32 {
    let Some(result) = (unsafe { with_host_path(path, |native| std::fs::read_dir(native)) }) else {
        return ABI_HANDLE_FAILURE as i32;
    };
    let Ok(directory) = result else { return ABI_HANDLE_FAILURE as i32 };
    let handle = NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if handle <= 0 {
        return ABI_HANDLE_FAILURE as i32;
    }
    let Ok(mut table) = directories().lock() else { return ABI_HANDLE_FAILURE as i32 };
    table.insert(handle, directory);
    handle
}

/// Writes one native entry name into caller-owned path storage.
///
/// # Safety
/// `storage` must point to a valid writable `ActusBuffer`; `handle` must be a
/// live token returned by `actus_directory_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_directory_next(handle: i32, storage: BufferHandle) -> i32 {
    let Some(output) = (unsafe { writable_buffer(storage) }) else {
        return ABI_STATUS_FAILURE;
    };
    let Ok(mut table) = directories().lock() else { return ABI_STATUS_FAILURE };
    let Some(directory) = table.get_mut(&handle) else { return ABI_STATUS_FAILURE };
    let next = directory.next();
    let Some(entry) = next else { return ABI_STATUS_END_OF_STREAM };
    let Ok(entry) = entry else { return ABI_STATUS_FAILURE };
    let name = entry.file_name();
    unsafe { write_entry_name(output, &name) }
}

/// Closes one directory token and releases its host iterator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_directory_close(handle: i32) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    let Ok(mut table) = directories().lock() else { return ABI_STATUS_FAILURE };
    table.remove(&handle).map_or(ABI_STATUS_FAILURE, |_| 0)
}

unsafe fn writable_buffer(handle: BufferHandle) -> Option<&'static mut ActusBuffer> {
    if handle.is_null() {
        return None;
    }
    let buffer = unsafe { &mut *handle };
    if buffer.length > buffer.capacity || (buffer.capacity > 0 && buffer.data.is_null()) {
        return None;
    }
    Some(buffer)
}

unsafe fn write_entry_name(output: &mut ActusBuffer, name: &OsStr) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = name.as_bytes();
        let required = bytes.len().saturating_add(1);
        if required > output.capacity || output.data.is_null() {
            return DIRECTORY_STATUS_BUFFER_TOO_SMALL;
        }
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), output.data, bytes.len()) };
        unsafe { *output.data.add(bytes.len()) = 0 };
        output.length = required;
        return 0;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units = name.encode_wide().collect::<Vec<_>>();
        let required = units.len().saturating_add(1).saturating_mul(2);
        if required > output.capacity || output.data.is_null() {
            return DIRECTORY_STATUS_BUFFER_TOO_SMALL;
        }
        for (index, unit) in units.iter().enumerate() {
            let bytes = unit.to_ne_bytes();
            unsafe {
                *output.data.add(index * 2) = bytes[0];
                *output.data.add(index * 2 + 1) = bytes[1];
            }
        }
        unsafe {
            *output.data.add(units.len() * 2) = 0;
            *output.data.add(units.len() * 2 + 1) = 0;
        }
        output.length = required;
        return 1;
    }
    #[allow(unreachable_code)]
    ABI_STATUS_FAILURE
}

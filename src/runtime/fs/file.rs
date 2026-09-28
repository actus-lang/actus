use std::io::{Read, Seek, SeekFrom, Write};

use super::super::contract::{ABI_HANDLE_FAILURE, ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS};

const MODE_CREATE: u32 = 1;
const WHENCE_START: i32 = 0;
const WHENCE_CURRENT: i32 = 1;
const WHENCE_END: i32 = 2;

/// Opens a host file using the stable pointer-and-length C ABI.
///
/// # Safety
/// `path` must reference `path_len` readable bytes or be null with a zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open(path: *const u8, path_len: usize, mode: u32) -> i64 {
    if path.is_null() {
        return ABI_HANDLE_FAILURE;
    }
    let bytes = unsafe { std::slice::from_raw_parts(path, path_len) };
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    if mode == MODE_CREATE {
        options.write(true).create(true).truncate(true);
    }
    open_with_options(bytes, options)
}

/// Reads into a caller-owned raw memory region.
///
/// # Safety
/// `buf` must reference `len` writable bytes or be null when `len` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_read(handle: i64, buf: *mut u8, len: usize) -> i64 {
    if handle < 0 || (len > 0 && buf.is_null()) {
        return ABI_STATUS_FAILURE as i64;
    }
    let target =
        if len == 0 { &mut [] } else { unsafe { std::slice::from_raw_parts_mut(buf, len) } };
    with_file(handle, |file| {
        file.read(target).map_or(ABI_STATUS_FAILURE as i64, |count| count as i64)
    })
}

/// Writes from a caller-owned raw memory region.
///
/// # Safety
/// `buf` must reference `len` readable bytes or be null when `len` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_write(handle: i64, buf: *const u8, len: usize) -> i64 {
    if handle < 0 || (len > 0 && buf.is_null()) {
        return ABI_STATUS_FAILURE as i64;
    }
    let source = if len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(buf, len) } };
    with_file(handle, |file| {
        file.write(source).map_or(ABI_STATUS_FAILURE as i64, |count| count as i64)
    })
}

/// Flushes a host file, returning zero or `-1`.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_flush(handle: i64) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    with_file(handle, |file| {
        if file.flush().is_ok() {
            i64::from(ABI_STATUS_SUCCESS)
        } else {
            i64::from(ABI_STATUS_FAILURE)
        }
    }) as i32
}

/// Closes a host file, returning zero or `-1`.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_close(handle: i64) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    close_file(handle)
}

/// Moves a host file cursor and returns its resulting absolute position.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_seek(handle: i64, offset: i64, whence: i32) -> i64 {
    if handle < 0 {
        return ABI_STATUS_FAILURE as i64;
    }
    let origin = match whence {
        WHENCE_START if offset >= 0 => SeekFrom::Start(offset as u64),
        WHENCE_START => return ABI_STATUS_FAILURE as i64,
        WHENCE_CURRENT => SeekFrom::Current(offset),
        WHENCE_END => SeekFrom::End(offset),
        _ => return ABI_STATUS_FAILURE as i64,
    };
    with_file(handle, |file| {
        file.seek(origin).map_or(ABI_STATUS_FAILURE as i64, |position| position as i64)
    })
}

/// Moves a file cursor through the scalar ABI used by Actus external verbs.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_seek_buffer(handle: i32, offset: i32, whence: i32) -> i32 {
    let position = actus_file_seek(actus_handle(handle), i64::from(offset), whence);
    i32::try_from(position).unwrap_or(ABI_STATUS_FAILURE)
}

pub(crate) fn handle_to_actus(handle: i64) -> i32 {
    if handle < 0 {
        return ABI_HANDLE_FAILURE as i32;
    }
    #[cfg(unix)]
    {
        return i32::try_from(handle).unwrap_or(ABI_HANDLE_FAILURE as i32);
    }
    #[cfg(windows)]
    {
        return windows_handles::insert(handle);
    }
    #[allow(unreachable_code)]
    {
        ABI_HANDLE_FAILURE as i32
    }
}

pub(super) fn open_with_options(bytes: &[u8], options: std::fs::OpenOptions) -> i64 {
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
            .map_or(-1, |file| file.into_raw_handle() as i64);
    }
    #[allow(unreachable_code)]
    -1
}

pub(crate) fn actus_handle(handle: i32) -> i64 {
    if handle < 0 {
        return ABI_HANDLE_FAILURE;
    }
    #[cfg(unix)]
    {
        return i64::from(handle);
    }
    #[cfg(windows)]
    {
        return windows_handles::get(handle).unwrap_or(ABI_HANDLE_FAILURE);
    }
    #[allow(unreachable_code)]
    ABI_HANDLE_FAILURE
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
    ABI_STATUS_FAILURE
}

pub(crate) fn with_file(handle: i64, operation: impl FnOnce(&mut std::fs::File) -> i64) -> i64 {
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

use std::io::{Read, Seek, SeekFrom, Write};

use super::super::contract::{ABI_HANDLE_FAILURE, ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS};

const MODE_CREATE: u32 = 1;
const WHENCE_START: i32 = 0;
const WHENCE_CURRENT: i32 = 1;
const WHENCE_END: i32 = 2;

pub(crate) const FILE_STATUS_BUSY: i32 = -3;
pub(crate) const FILE_STATUS_UNSUPPORTED: i32 = -2;

#[cfg(unix)]
const LOCK_SH: i32 = 1;
#[cfg(unix)]
const LOCK_EX: i32 = 2;
#[cfg(unix)]
const LOCK_NB: i32 = 4;
#[cfg(unix)]
const LOCK_UN: i32 = 8;

#[cfg(unix)]
unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

#[cfg(windows)]
#[repr(C)]
struct WindowsOverlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LockFileEx(
        file: *mut std::ffi::c_void,
        flags: u32,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut WindowsOverlapped,
    ) -> i32;
    fn UnlockFileEx(
        file: *mut std::ffi::c_void,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut WindowsOverlapped,
    ) -> i32;
}

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
    #[cfg(actus_test_faults)]
    if super::test_faults::take_fault(super::test_faults::FAULT_FLUSH_FAILURE) {
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

/// Synchronizes file contents through the host file descriptor.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_sync_data(handle: i64) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    with_file(handle, |file| {
        if file.sync_data().is_ok() {
            i64::from(ABI_STATUS_SUCCESS)
        } else {
            i64::from(ABI_STATUS_FAILURE)
        }
    }) as i32
}

/// Synchronizes file contents and metadata through the host file descriptor.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_sync_all(handle: i64) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    with_file(handle, |file| {
        if file.sync_all().is_ok() {
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
    #[cfg(actus_test_faults)]
    if super::test_faults::take_fault(super::test_faults::FAULT_CLOSE_FAILURE) {
        return ABI_STATUS_FAILURE;
    }
    close_file(handle)
}

/// Acquires a shared advisory lock on the complete file.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_lock_shared(handle: i64) -> i32 {
    advisory_lock(native_lock_handle(handle), false, false)
}

/// Acquires an exclusive advisory lock on the complete file.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_lock_exclusive(handle: i64) -> i32 {
    advisory_lock(native_lock_handle(handle), true, false)
}

/// Attempts an exclusive advisory lock without waiting for another owner.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_try_lock(handle: i64) -> i32 {
    advisory_lock(native_lock_handle(handle), true, true)
}

/// Releases the advisory lock associated with the complete file.
#[unsafe(no_mangle)]
pub extern "C" fn actus_file_unlock(handle: i64) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    #[cfg(unix)]
    {
        return if unsafe { flock(handle as i32, LOCK_UN) } == 0 {
            ABI_STATUS_SUCCESS
        } else {
            map_lock_error()
        };
    }
    #[cfg(windows)]
    {
        let native = actus_handle(i32::try_from(handle).unwrap_or(-1));
        let identity =
            match super::identity::windows_file_identity_handle(native as *mut std::ffi::c_void) {
                Ok(identity) => identity,
                Err(_) => return ABI_STATUS_FAILURE,
            };
        let mut overlapped = WindowsOverlapped {
            internal: 0,
            internal_high: 0,
            offset: 0,
            offset_high: 0,
            event: std::ptr::null_mut(),
        };
        return if unsafe {
            UnlockFileEx(native as *mut std::ffi::c_void, 0, u32::MAX, u32::MAX, &mut overlapped)
        } == 0
        {
            map_lock_error()
        } else {
            super::locking::remove(identity, handle);
            ABI_STATUS_SUCCESS
        };
    }
    #[allow(unreachable_code)]
    FILE_STATUS_UNSUPPORTED
}

fn native_lock_handle(handle: i64) -> i64 {
    #[cfg(unix)]
    {
        return handle;
    }
    #[cfg(windows)]
    {
        return actus_handle(i32::try_from(handle).unwrap_or(-1));
    }
    #[allow(unreachable_code)]
    -1
}

fn advisory_lock(handle: i64, exclusive: bool, non_blocking: bool) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    #[cfg(unix)]
    {
        let mut operation = if exclusive { LOCK_EX } else { LOCK_SH };
        if non_blocking {
            operation |= LOCK_NB;
        }
        return if unsafe { flock(handle as i32, operation) } == 0 {
            ABI_STATUS_SUCCESS
        } else {
            map_lock_error()
        };
    }
    #[cfg(windows)]
    {
        return windows_advisory_lock(handle, exclusive, non_blocking);
    }
    #[allow(unreachable_code)]
    FILE_STATUS_UNSUPPORTED
}

#[cfg(windows)]
fn windows_advisory_lock(handle: i64, exclusive: bool, non_blocking: bool) -> i32 {
    let identity = match super::identity::windows_file_identity_handle(handle as *mut _) {
        Ok(identity) => identity,
        Err(_) => return ABI_STATUS_FAILURE,
    };
    if !super::locking::register(identity, handle, exclusive) {
        return FILE_STATUS_BUSY;
    }
    let mut flags = if exclusive { 2 } else { 0 };
    if non_blocking {
        flags |= 1;
    }
    let mut overlapped = WindowsOverlapped {
        internal: 0,
        internal_high: 0,
        offset: 0,
        offset_high: 0,
        event: std::ptr::null_mut(),
    };
    let status =
        if unsafe { LockFileEx(handle as *mut _, flags, 0, u32::MAX, u32::MAX, &mut overlapped) }
            == 0
        {
            map_lock_error()
        } else {
            ABI_STATUS_SUCCESS
        };
    if status != ABI_STATUS_SUCCESS {
        super::locking::remove(identity, handle);
    }
    status
}

fn map_lock_error() -> i32 {
    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
        FILE_STATUS_BUSY
    } else {
        ABI_STATUS_FAILURE
    }
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
            .map_or(ABI_HANDLE_FAILURE, |file| file.into_raw_fd() as i64);
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        use std::os::windows::io::IntoRawHandle;
        let wide = String::from_utf8_lossy(bytes).encode_utf16().collect::<Vec<_>>();
        return options
            .open(std::ffi::OsString::from_wide(&wide))
            .map_or(ABI_HANDLE_FAILURE, |file| file.into_raw_handle() as i64);
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
    pub fn remove(id: i32) {
        if let Ok(mut handles) = table().lock() {
            handles.remove(&id);
        }
    }
}

#[cfg(windows)]
pub(crate) fn remove_actus_handle(handle: i32) {
    windows_handles::remove(handle);
}

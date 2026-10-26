use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::ReadDir;
use std::sync::{Mutex, OnceLock};

use super::super::contract::{ABI_HANDLE_FAILURE, ABI_STATUS_END_OF_STREAM, ABI_STATUS_FAILURE};
use super::super::path::ActusPath;
use super::super::types::{ActusBuffer, ActusMetadata, BufferHandle};
use super::operations::write_metadata;
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

/// Writes one entry name and a symlink-preserving metadata snapshot.
///
/// # Safety
/// `storage` must point to a valid writable `ActusBuffer`, `metadata` must
/// point to writable `ActusMetadata`, and `handle` must be a live token
/// returned by `actus_directory_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_directory_next_entry(
    handle: i32,
    storage: BufferHandle,
    metadata: *mut ActusMetadata,
) -> i32 {
    let Some(output) = (unsafe { writable_buffer(storage) }) else {
        return ABI_STATUS_FAILURE;
    };
    if metadata.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let Ok(mut table) = directories().lock() else { return ABI_STATUS_FAILURE };
    let Some(directory) = table.get_mut(&handle) else { return ABI_STATUS_FAILURE };
    let next = directory.next();
    let Some(entry) = next else { return ABI_STATUS_END_OF_STREAM };
    let Ok(entry) = entry else { return ABI_STATUS_FAILURE };
    let Ok(snapshot) = entry.path().symlink_metadata() else { return ABI_STATUS_FAILURE };
    let name_status = unsafe { write_entry_name(output, &entry.file_name()) };
    if name_status < 0 {
        return name_status;
    }
    unsafe { write_metadata(metadata, &snapshot) };
    name_status
}

/// Closes one directory token and releases its host iterator.
///
/// # Safety
/// `handle` must be a directory token returned by `actus_directory_open` or a
/// non-negative token whose lifetime is independently controlled by the host.
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::runtime::path::ActusPath;

    fn empty_metadata() -> ActusMetadata {
        ActusMetadata {
            size: 0,
            is_file: 0,
            is_dir: 0,
            readonly: 0,
            is_symlink: 0,
            modified_seconds: 0,
            modified_status: 0,
            accessed_seconds: 0,
            accessed_status: 0,
            created_seconds: 0,
            created_status: 0,
            identity_primary: 0,
            identity_secondary: 0,
            identity_kind: 0,
            identity_status: 0,
        }
    }

    #[test]
    fn directory_next_entry_writes_file_classification() {
        use std::os::unix::ffi::OsStrExt;

        let root =
            std::env::temp_dir().join(format!("actus-directory-entry-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary directory should be created");
        let file = root.join("entry");
        std::fs::write(&file, b"entry").expect("temporary entry should be created");
        let mut path_bytes = root.as_os_str().as_bytes().to_vec();
        let path_length = path_bytes.len();
        path_bytes.push(0);
        let mut path_buffer = ActusBuffer {
            data: path_bytes.as_mut_ptr(),
            length: path_bytes.len(),
            capacity: path_bytes.capacity(),
        };
        let path = ActusPath {
            storage: &mut path_buffer,
            platform: 0,
            length: i32::try_from(path_length).expect("temporary path should fit ABI"),
            capacity: i32::try_from(path_length).expect("temporary path should fit ABI"),
            terminated: 1,
        };
        let handle = unsafe { actus_directory_open(&path) };
        assert!(handle > 0);
        let mut entry_bytes = vec![0u8; 4096];
        let mut entry_buffer =
            ActusBuffer { data: entry_bytes.as_mut_ptr(), length: 0, capacity: entry_bytes.len() };
        let mut metadata = empty_metadata();
        let status =
            unsafe { actus_directory_next_entry(handle, &mut entry_buffer, &mut metadata) };
        assert_eq!(status, 0);
        assert_eq!(metadata.is_file, 1);
        assert_eq!(metadata.is_dir, 0);
        assert_eq!(metadata.is_symlink, 0);
        assert_eq!(metadata.modified_status, 0);
        assert_eq!(metadata.accessed_status, 0);
        assert_eq!(metadata.identity_kind, 1);
        assert_eq!(metadata.identity_status, 0);
        assert_eq!(unsafe { actus_directory_close(handle) }, 0);
        std::fs::remove_file(file).expect("temporary entry should be removed");
        std::fs::remove_dir(root).expect("temporary directory should be removed");
    }

    #[test]
    fn directory_close_releases_native_token_exactly_once() {
        let before = directories().lock().expect("directory table should lock").len();
        let mut bytes = b".\0".to_vec();
        let mut buffer = ActusBuffer {
            data: bytes.as_mut_ptr(),
            length: bytes.len(),
            capacity: bytes.capacity(),
        };
        let path =
            ActusPath { storage: &mut buffer, platform: 0, length: 1, capacity: 1, terminated: 1 };
        let handle = unsafe { actus_directory_open(&path) };
        assert!(handle > 0);
        assert_eq!(directories().lock().expect("directory table should lock").len(), before + 1);
        assert_eq!(unsafe { actus_directory_close(handle) }, 0);
        assert_eq!(directories().lock().expect("directory table should lock").len(), before);
        assert_eq!(unsafe { actus_directory_close(handle) }, ABI_STATUS_FAILURE);
        let mut output = ActusBuffer { data: std::ptr::null_mut(), length: 0, capacity: 0 };
        assert_eq!(unsafe { actus_directory_next(handle, &mut output) }, ABI_STATUS_FAILURE);
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use crate::runtime::path::ActusPath;

    #[test]
    fn directory_close_releases_native_token_exactly_once() {
        let before = directories().lock().expect("directory table should lock").len();
        let mut units = vec![46u16, 0u16];
        let mut buffer = ActusBuffer {
            data: units.as_mut_ptr().cast(),
            length: units.len() * 2,
            capacity: units.capacity() * 2,
        };
        let path =
            ActusPath { storage: &mut buffer, platform: 1, length: 1, capacity: 1, terminated: 1 };
        let handle = unsafe { actus_directory_open(&path) };
        assert!(handle > 0);
        assert_eq!(directories().lock().expect("directory table should lock").len(), before + 1);
        assert_eq!(unsafe { actus_directory_close(handle) }, 0);
        assert_eq!(directories().lock().expect("directory table should lock").len(), before);
        assert_eq!(unsafe { actus_directory_close(handle) }, ABI_STATUS_FAILURE);
        let mut output = ActusBuffer { data: std::ptr::null_mut(), length: 0, capacity: 0 };
        assert_eq!(unsafe { actus_directory_next(handle, &mut output) }, ABI_STATUS_FAILURE);
    }
}

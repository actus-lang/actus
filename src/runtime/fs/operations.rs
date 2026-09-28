use super::super::contract::{ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS};
use super::super::path::ActusPath;
use super::super::types::{ActusMetadata, BufferHandle};
use super::path_bridge::{with_host_path, with_host_paths};
use super::{actus_handle, with_file};

/// Writes file metadata into a caller-owned C-layout output struct.
///
/// # Safety
/// `path` must be null or a valid live `ActusBuffer`; `output` must point to a
/// writable `ActusMetadata` for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_metadata_buffer(
    path: BufferHandle,
    output: *mut ActusMetadata,
) -> i32 {
    if path.is_null() || output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let path = unsafe { &*path };
    if path.length > 0 && path.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let bytes = unsafe { buffer_bytes(path) };
    let Ok(metadata) = host_path(bytes).metadata() else { return ABI_STATUS_FAILURE };
    unsafe { write_metadata(output, &metadata) }
}

/// Writes metadata for a validated borrowed `ActusPath`.
///
/// # Safety
/// `path` must be a valid immutable path descriptor and `output` must be
/// writable for one `ActusMetadata`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_metadata_path(
    path: *const ActusPath,
    output: *mut ActusMetadata,
) -> i32 {
    if output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let result = unsafe { with_host_path(path, |native| native.metadata()) };
    match result {
        Some(Ok(metadata)) => unsafe { write_metadata(output, &metadata) },
        _ => ABI_STATUS_FAILURE,
    }
}

/// Writes metadata for an already-open file handle into an output struct.
///
/// # Safety
/// `output` must point to writable `ActusMetadata` storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_metadata_handle_buffer(
    handle: i32,
    output: *mut ActusMetadata,
) -> i32 {
    if output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    with_file(actus_handle(handle), |file| {
        file.metadata().map_or(ABI_STATUS_FAILURE as i64, |metadata| unsafe {
            write_metadata(output, &metadata) as i64
        })
    }) as i32
}

/// Removes a regular file represented by an Actus path buffer.
///
/// # Safety
/// `path` must point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_remove_buffer(path: BufferHandle) -> i32 {
    unsafe { path_operation(path, |value| std::fs::remove_file(value)) }
}

/// Removes a regular file named by a validated borrowed `ActusPath`.
///
/// # Safety
/// `path` must be a valid immutable path descriptor for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_remove_path(path: *const ActusPath) -> i32 {
    unsafe { with_host_path(path, |value| std::fs::remove_file(value)) }
        .and_then(Result::ok)
        .map_or(ABI_STATUS_FAILURE, |_| ABI_STATUS_SUCCESS)
}

/// Renames a path using the host filesystem's atomic rename operation.
///
/// # Safety
/// `from` and `to` must point to valid live `ActusBuffer` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_rename_buffer(from: BufferHandle, to: BufferHandle) -> i32 {
    unsafe {
        two_path_operation(from, to, |source, destination| std::fs::rename(source, destination))
    }
}

/// Renames two validated borrowed `ActusPath` values.
///
/// # Safety
/// Both paths must be valid immutable descriptors for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_rename_path(
    from: *const ActusPath,
    to: *const ActusPath,
) -> i32 {
    unsafe { with_host_paths(from, to, |source, destination| std::fs::rename(source, destination)) }
        .and_then(Result::ok)
        .map_or(ABI_STATUS_FAILURE, |_| ABI_STATUS_SUCCESS)
}

/// Copies a file and returns the number of copied bytes, or `-1` on failure.
///
/// # Safety
/// `from` and `to` must point to valid live `ActusBuffer` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_copy_buffer(from: BufferHandle, to: BufferHandle) -> i32 {
    unsafe {
        two_path_operation_value(from, to, |source, destination| std::fs::copy(source, destination))
    }
}

/// Copies a file between two validated borrowed `ActusPath` values.
///
/// # Safety
/// Both paths must be valid immutable descriptors for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_copy_path(from: *const ActusPath, to: *const ActusPath) -> i32 {
    unsafe { with_host_paths(from, to, |source, destination| std::fs::copy(source, destination)) }
        .and_then(Result::ok)
        .map_or(ABI_STATUS_FAILURE, |count| i32::try_from(count).unwrap_or(ABI_STATUS_FAILURE))
}

/// Creates one directory at the requested path.
///
/// # Safety
/// `path` must point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_create_dir_buffer(path: BufferHandle) -> i32 {
    unsafe { path_operation(path, |value| std::fs::create_dir(value)) }
}

/// Creates one directory named by a validated borrowed `ActusPath`.
///
/// # Safety
/// `path` must be a valid immutable path descriptor for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_create_dir_path(path: *const ActusPath) -> i32 {
    unsafe { with_host_path(path, |value| std::fs::create_dir(value)) }
        .and_then(Result::ok)
        .map_or(ABI_STATUS_FAILURE, |_| ABI_STATUS_SUCCESS)
}

/// Removes one empty directory at the requested path.
///
/// # Safety
/// `path` must point to a valid live `ActusBuffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_remove_dir_buffer(path: BufferHandle) -> i32 {
    unsafe { path_operation(path, |value| std::fs::remove_dir(value)) }
}

/// Removes one empty directory named by a validated borrowed `ActusPath`.
///
/// # Safety
/// `path` must be a valid immutable path descriptor for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_remove_dir_path(path: *const ActusPath) -> i32 {
    unsafe { with_host_path(path, |value| std::fs::remove_dir(value)) }
        .and_then(Result::ok)
        .map_or(ABI_STATUS_FAILURE, |_| ABI_STATUS_SUCCESS)
}

unsafe fn buffer_bytes(buffer: &super::super::types::ActusBuffer) -> &[u8] {
    if buffer.length == 0 {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) }
}

fn host_path(bytes: &[u8]) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        return std::path::PathBuf::from(std::ffi::OsStr::from_bytes(bytes));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let wide = String::from_utf8_lossy(bytes).encode_utf16().collect::<Vec<_>>();
        return std::path::PathBuf::from(std::ffi::OsString::from_wide(&wide));
    }
    #[allow(unreachable_code)]
    std::path::PathBuf::new()
}

unsafe fn write_metadata(output: *mut ActusMetadata, metadata: &std::fs::Metadata) -> i32 {
    let Ok(size) = i32::try_from(metadata.len()) else { return ABI_STATUS_FAILURE };
    unsafe {
        *output = ActusMetadata {
            size,
            is_file: i32::from(metadata.is_file()),
            is_dir: i32::from(metadata.is_dir()),
            readonly: i32::from(metadata.permissions().readonly()),
        };
    }
    ABI_STATUS_SUCCESS
}

unsafe fn path_operation<F>(path: BufferHandle, operation: F) -> i32
where
    F: FnOnce(&std::path::Path) -> std::io::Result<()>,
{
    if path.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let path = unsafe { &*path };
    if path.length > 0 && path.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    operation(&host_path(unsafe { buffer_bytes(path) }))
        .map_or(ABI_STATUS_FAILURE, |_| ABI_STATUS_SUCCESS)
}

unsafe fn two_path_operation<F>(from: BufferHandle, to: BufferHandle, operation: F) -> i32
where
    F: FnOnce(&std::path::Path, &std::path::Path) -> std::io::Result<()>,
{
    let Some((from, to)) = (unsafe { path_pair(from, to) }) else { return ABI_STATUS_FAILURE };
    operation(&from, &to).map_or(ABI_STATUS_FAILURE, |_| ABI_STATUS_SUCCESS)
}

unsafe fn two_path_operation_value<F>(from: BufferHandle, to: BufferHandle, operation: F) -> i32
where
    F: FnOnce(&std::path::Path, &std::path::Path) -> std::io::Result<u64>,
{
    let Some((from, to)) = (unsafe { path_pair(from, to) }) else { return ABI_STATUS_FAILURE };
    operation(&from, &to)
        .map_or(ABI_STATUS_FAILURE, |count| i32::try_from(count).unwrap_or(ABI_STATUS_FAILURE))
}

unsafe fn path_pair(
    from: BufferHandle,
    to: BufferHandle,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    if from.is_null() || to.is_null() {
        return None;
    }
    let from = unsafe { &*from };
    let to = unsafe { &*to };
    if (from.length > 0 && from.data.is_null()) || (to.length > 0 && to.data.is_null()) {
        return None;
    }
    Some((host_path(unsafe { buffer_bytes(from) }), host_path(unsafe { buffer_bytes(to) })))
}

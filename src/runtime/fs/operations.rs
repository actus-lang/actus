use super::super::contract::{ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS};
use super::super::path::ActusPath;
use super::super::types::{ActusMetadata, BufferHandle};
use super::path_bridge::{replace_output_path, with_host_path, with_host_paths};
use super::{actus_handle, with_file};

pub(crate) const METADATA_VALUE_AVAILABLE: i32 = 0;
pub(crate) const METADATA_VALUE_UNSUPPORTED: i32 = 1;
pub(crate) const METADATA_VALUE_UNAVAILABLE: i32 = 2;
pub(crate) const METADATA_IDENTITY_NONE: i32 = 0;
pub(crate) const METADATA_IDENTITY_POSIX: i32 = 1;
#[cfg(windows)]
pub(crate) const METADATA_IDENTITY_WINDOWS: i32 = 2;
const METADATA_STATUS_NOT_FOUND: i32 = -2;
const METADATA_STATUS_PERMISSION_DENIED: i32 = -3;
const METADATA_STATUS_UNSUPPORTED: i32 = -4;
const RENAME_STATUS_CROSS_DEVICE: i32 = -2;

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
    match host_path(bytes).metadata() {
        Ok(metadata) => unsafe { write_metadata(output, &metadata) },
        Err(error) => metadata_error_status(&error),
    }
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
        Some(Err(error)) => metadata_error_status(&error),
        None => ABI_STATUS_FAILURE,
    }
}

/// Writes metadata for a validated path without following a symbolic link.
///
/// # Safety
/// `path` must be a valid immutable path descriptor and `output` must be
/// writable for one `ActusMetadata`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_symlink_metadata_path(
    path: *const ActusPath,
    output: *mut ActusMetadata,
) -> i32 {
    if output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let result = unsafe { with_host_path(path, |native| native.symlink_metadata()) };
    match result {
        Some(Ok(metadata)) => unsafe { write_metadata(output, &metadata) },
        Some(Err(error)) => metadata_error_status(&error),
        None => ABI_STATUS_FAILURE,
    }
}

fn metadata_error_status(error: &std::io::Error) -> i32 {
    match error.kind() {
        std::io::ErrorKind::NotFound => METADATA_STATUS_NOT_FOUND,
        std::io::ErrorKind::PermissionDenied => METADATA_STATUS_PERMISSION_DENIED,
        std::io::ErrorKind::Unsupported => METADATA_STATUS_UNSUPPORTED,
        _ => ABI_STATUS_FAILURE,
    }
}

/// Resolves one validated path through the host filesystem and writes the
/// resulting identity path into an owned Actus destination.
///
/// # Safety
/// `path` must be a valid immutable path and `output` must be writable for
/// the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_canonicalize_path(
    path: *const ActusPath,
    output: *mut ActusPath,
) -> i32 {
    if output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let result = unsafe { with_host_path(path, |value| std::fs::canonicalize(value)) };
    match result {
        Some(Ok(canonical)) if unsafe { replace_output_path(output, &canonical) } => {
            super::super::contract::ABI_STATUS_SUCCESS
        }
        _ => ABI_STATUS_FAILURE,
    }
}

/// Compares two validated paths using the host's stable filesystem identity.
///
/// # Safety
/// Both paths must be valid immutable descriptors for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_same_path(
    left: *const ActusPath,
    right: *const ActusPath,
) -> i32 {
    let result = unsafe { with_host_paths(left, right, same_file_identity) };
    match result {
        Some(Ok(true)) => 1,
        Some(Ok(false)) => 0,
        Some(Err(IdentityError::Unsupported)) => -2,
        _ => ABI_STATUS_FAILURE,
    }
}

pub(super) enum IdentityError {
    Failed,
    Unsupported,
}

pub(super) fn same_file_identity(
    left: &std::path::Path,
    right: &std::path::Path,
) -> Result<bool, IdentityError> {
    let left = std::fs::metadata(left).map_err(|_| IdentityError::Failed)?;
    let right = std::fs::metadata(right).map_err(|_| IdentityError::Failed)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return Ok(left.dev() == right.dev() && left.ino() == right.ino());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let Some(left_volume) = left.volume_serial_number() else {
            return Err(IdentityError::Unsupported);
        };
        let Some(right_volume) = right.volume_serial_number() else {
            return Err(IdentityError::Unsupported);
        };
        let Some(left_index) = left.file_index() else {
            return Err(IdentityError::Unsupported);
        };
        let Some(right_index) = right.file_index() else {
            return Err(IdentityError::Unsupported);
        };
        return Ok(left_volume == right_volume && left_index == right_index);
    }
    #[allow(unreachable_code)]
    Err(IdentityError::Unsupported)
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
    unsafe { two_path_rename_operation(from, to) }
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
    match unsafe {
        with_host_paths(from, to, |source, destination| std::fs::rename(source, destination))
    } {
        Some(Ok(())) => ABI_STATUS_SUCCESS,
        Some(Err(error)) => rename_error_status(&error),
        None => ABI_STATUS_FAILURE,
    }
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

/// Creates a directory and all missing parents at a validated path.
///
/// # Safety
/// `path` must be a valid immutable path descriptor for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_create_dir_all_path(path: *const ActusPath) -> i32 {
    unsafe { with_host_path(path, |value| std::fs::create_dir_all(value)) }
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

/// Removes a directory tree at a validated path.
///
/// # Safety
/// `path` must be a valid immutable path descriptor for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_remove_dir_all_path(path: *const ActusPath) -> i32 {
    unsafe { with_host_path(path, |value| std::fs::remove_dir_all(value)) }
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

pub(crate) unsafe fn write_metadata(
    output: *mut ActusMetadata,
    metadata: &std::fs::Metadata,
) -> i32 {
    let Ok(size) = i32::try_from(metadata.len()) else { return ABI_STATUS_FAILURE };
    let modified = metadata_timestamp(metadata.modified());
    let accessed = metadata_timestamp(metadata.accessed());
    let created = metadata_timestamp(metadata.created());
    let identity = metadata_identity(metadata);
    unsafe {
        *output = ActusMetadata {
            size,
            is_file: i32::from(metadata.is_file()),
            is_dir: i32::from(metadata.is_dir()),
            readonly: i32::from(metadata.permissions().readonly()),
            is_symlink: i32::from(metadata.file_type().is_symlink()),
            modified_seconds: modified.0,
            modified_status: modified.1,
            accessed_seconds: accessed.0,
            accessed_status: accessed.1,
            created_seconds: created.0,
            created_status: created.1,
            identity_primary: identity.0,
            identity_secondary: identity.1,
            identity_kind: identity.2,
            identity_status: identity.3,
        };
    }
    ABI_STATUS_SUCCESS
}

fn metadata_timestamp(value: std::io::Result<std::time::SystemTime>) -> (i64, i32) {
    let Ok(value) = value else { return (0, METADATA_VALUE_UNSUPPORTED) };
    let Ok(duration) = value.duration_since(std::time::UNIX_EPOCH) else {
        return (0, METADATA_VALUE_UNAVAILABLE);
    };
    let Ok(seconds) = i64::try_from(duration.as_secs()) else {
        return (0, METADATA_VALUE_UNAVAILABLE);
    };
    (seconds, METADATA_VALUE_AVAILABLE)
}

fn metadata_identity(metadata: &std::fs::Metadata) -> (u64, u64, i32, i32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return (metadata.dev(), metadata.ino(), METADATA_IDENTITY_POSIX, METADATA_VALUE_AVAILABLE);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let Some(volume) = metadata.volume_serial_number() else {
            return (0, 0, METADATA_IDENTITY_WINDOWS, METADATA_VALUE_UNSUPPORTED);
        };
        let Some(index) = metadata.file_index() else {
            return (0, 0, METADATA_IDENTITY_WINDOWS, METADATA_VALUE_UNSUPPORTED);
        };
        return (u64::from(volume), index, METADATA_IDENTITY_WINDOWS, METADATA_VALUE_AVAILABLE);
    }
    #[allow(unreachable_code)]
    (0, 0, METADATA_IDENTITY_NONE, METADATA_VALUE_UNSUPPORTED)
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

unsafe fn two_path_rename_operation(from: BufferHandle, to: BufferHandle) -> i32 {
    let Some((from, to)) = (unsafe { path_pair(from, to) }) else { return ABI_STATUS_FAILURE };
    std::fs::rename(&from, &to)
        .map_or_else(|error| rename_error_status(&error), |_| ABI_STATUS_SUCCESS)
}

fn rename_error_status(error: &std::io::Error) -> i32 {
    match error.kind() {
        std::io::ErrorKind::CrossesDevices => RENAME_STATUS_CROSS_DEVICE,
        _ => ABI_STATUS_FAILURE,
    }
}

pub(super) unsafe fn path_pair(
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

#[cfg(test)]
mod tests {
    use super::{RENAME_STATUS_CROSS_DEVICE, rename_error_status};

    #[test]
    fn rename_preserves_cross_device_error_classification() {
        let error = std::io::Error::from(std::io::ErrorKind::CrossesDevices);
        assert_eq!(rename_error_status(&error), RENAME_STATUS_CROSS_DEVICE);
    }

    #[test]
    fn rename_keeps_other_errors_generic() {
        let error = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert_eq!(rename_error_status(&error), super::ABI_STATUS_FAILURE);
    }
}

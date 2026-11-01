use super::super::contract::{ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS};
use super::super::path::ActusPath;
use super::super::types::ActusMetadata;
use super::path_bridge::{with_host_path, with_host_paths};

const PUBLICATION_STATUS_UNSUPPORTED: i32 = -2;
const PUBLICATION_STATUS_CONFLICT: i32 = -3;
const PUBLICATION_STATUS_BUSY: i32 = -4;

/// Replaces an existing path through the host's atomic rename primitive.
///
/// # Safety
/// `from` and `to` must be valid immutable `ActusPath` descriptors for the
/// duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_replace_atomic_path(
    from: *const ActusPath,
    to: *const ActusPath,
) -> i32 {
    let result = unsafe { with_host_paths(from, to, replace_atomic) };
    match result {
        Some(Ok(())) => ABI_STATUS_SUCCESS,
        Some(Err(error)) if error.kind() == std::io::ErrorKind::Unsupported => {
            PUBLICATION_STATUS_UNSUPPORTED
        }
        Some(Err(_)) | None => ABI_STATUS_FAILURE,
    }
}

fn replace_atomic(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        return std::fs::rename(from, to);
    }
    #[cfg(windows)]
    {
        return windows_replace_atomic(from, to);
    }
    #[allow(unreachable_code)]
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

#[cfg(windows)]
const REPLACEFILE_WRITE_THROUGH: u32 = 1;

#[cfg(windows)]
const MOVEFILE_REPLACE_EXISTING: u32 = 1;

#[cfg(windows)]
const MOVEFILE_WRITE_THROUGH: u32 = 8;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn ReplaceFileW(
        replaced_file_name: *const u16,
        replacement_file_name: *const u16,
        backup_file_name: *const u16,
        replace_flags: u32,
        exclude: *mut std::ffi::c_void,
        reserved: *mut std::ffi::c_void,
    ) -> i32;
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: u32) -> i32;
}

#[cfg(windows)]
fn windows_replace_atomic(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    let source = from.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    let destination = to.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();

    let replaced = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            source.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced != 0 {
        return Ok(());
    }

    let error = std::io::Error::last_os_error();
    if error.raw_os_error() != Some(2) {
        return Err(error);
    }

    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved != 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

/// Synchronizes the directory containing a validated path.
///
/// # Safety
/// `path` must be a valid immutable `ActusPath` for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_sync_parent_path(path: *const ActusPath) -> i32 {
    #[cfg(actus_test_faults)]
    if super::test_faults::take_fault(super::test_faults::FAULT_PARENT_OPEN_FAILURE) {
        return ABI_STATUS_FAILURE;
    }
    #[cfg(actus_test_faults)]
    if super::test_faults::take_fault(super::test_faults::FAULT_PARENT_SYNC_FAILURE) {
        return ABI_STATUS_FAILURE;
    }
    let result = unsafe { with_host_path(path, sync_parent_directory) };
    match result {
        Some(Ok(())) => ABI_STATUS_SUCCESS,
        Some(Err(error)) if error.kind() == std::io::ErrorKind::Unsupported => {
            PUBLICATION_STATUS_UNSUPPORTED
        }
        Some(Err(_)) | None => ABI_STATUS_FAILURE,
    }
}

/// Publishes `staging` only when `destination` still has the expected identity.
///
/// The comparison and replacement are serialized by an exclusive lock on
/// `lock_path`. This protects writers that use the same coordinator protocol;
/// callers must not describe it as protection from uncoordinated writers.
///
/// # Safety
/// All path and metadata pointers must remain valid for the duration of this
/// call, and `expected` must point to a readable `ActusMetadata` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_publish_coordinated_path(
    lock_path: *const ActusPath,
    staging: *const ActusPath,
    destination: *const ActusPath,
    expected: *const ActusMetadata,
) -> i32 {
    if expected.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let lock =
        unsafe { super::path_open::actus_file_open_path(lock_path, super::MODE_CREATE as i32) };
    if lock < 0 {
        return ABI_STATUS_FAILURE;
    }
    let locked = super::file::actus_file_lock_exclusive(i64::from(lock));
    if locked < 0 {
        let _ = super::file::actus_file_close(i64::from(lock));
        return match locked {
            -2 => PUBLICATION_STATUS_UNSUPPORTED,
            -3 => PUBLICATION_STATUS_BUSY,
            _ => ABI_STATUS_FAILURE,
        };
    }
    let expected = unsafe { &*expected };
    let identity_result =
        unsafe { with_host_path(destination, |path| compare_expected_identity(expected, path)) };
    let identity_matches = match identity_result {
        Some(Ok(value)) => value,
        Some(Err(status)) => {
            let _ = super::file::actus_file_unlock(i64::from(lock));
            let _ = super::file::actus_file_close(i64::from(lock));
            return status;
        }
        None => {
            let _ = super::file::actus_file_unlock(i64::from(lock));
            let _ = super::file::actus_file_close(i64::from(lock));
            return ABI_STATUS_FAILURE;
        }
    };
    if !identity_matches {
        let _ = super::file::actus_file_unlock(i64::from(lock));
        let _ = super::file::actus_file_close(i64::from(lock));
        return PUBLICATION_STATUS_CONFLICT;
    }
    let result = unsafe { with_host_paths(staging, destination, replace_atomic) };
    let status = match result {
        Some(Ok(())) => ABI_STATUS_SUCCESS,
        Some(Err(error)) if error.kind() == std::io::ErrorKind::Unsupported => {
            PUBLICATION_STATUS_UNSUPPORTED
        }
        Some(Err(_)) | None => ABI_STATUS_FAILURE,
    };
    let unlock_status = super::file::actus_file_unlock(i64::from(lock));
    let close_status = super::file::actus_file_close(i64::from(lock));
    if status == ABI_STATUS_SUCCESS && (unlock_status < 0 || close_status < 0) {
        return ABI_STATUS_FAILURE;
    }
    status
}

fn compare_expected_identity(
    expected: &ActusMetadata,
    path: &std::path::Path,
) -> Result<bool, i32> {
    let current = path.metadata();
    if expected.identity_kind == 0 {
        return match current {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
                Err(PUBLICATION_STATUS_UNSUPPORTED)
            }
            Err(_) => Err(ABI_STATUS_FAILURE),
        };
    }
    let current = match current {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
            return Err(PUBLICATION_STATUS_UNSUPPORTED);
        }
        Err(_) => return Err(ABI_STATUS_FAILURE),
    };
    let current_identity = native_identity(path, &current);
    Ok(current_identity
        == Some((expected.identity_primary, expected.identity_secondary, expected.identity_kind)))
}

fn native_identity(
    path: &std::path::Path,
    metadata: &std::fs::Metadata,
) -> Option<(u64, u64, i32)> {
    #[cfg(unix)]
    {
        let _ = path;
        use std::os::unix::fs::MetadataExt;
        return Some((metadata.dev(), metadata.ino(), 1));
    }
    #[cfg(windows)]
    {
        let _ = metadata;
        return super::identity::windows_file_identity(path)
            .ok()
            .map(|(primary, secondary)| (primary, secondary, 2));
    }
    #[allow(unreachable_code)]
    None
}

fn sync_parent_directory(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let parent = path.parent().filter(|value| !value.as_os_str().is_empty());
        let directory = parent.unwrap_or_else(|| std::path::Path::new("."));
        return std::fs::File::open(directory)?.sync_all();
    }
    #[cfg(windows)]
    {
        let _ = path;
        return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
    }
    #[allow(unreachable_code)]
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

#[cfg(test)]
mod tests {
    use super::{compare_expected_identity, sync_parent_directory};
    use crate::runtime::types::ActusMetadata;

    #[cfg(unix)]
    #[test]
    fn sync_parent_directory_accepts_existing_parent() {
        let path = std::env::temp_dir().join("actus-publication-test");
        sync_parent_directory(&path).expect("temporary directory should be syncable");
    }

    #[cfg(unix)]
    #[test]
    fn absent_identity_matches_a_missing_destination() {
        let path =
            std::env::temp_dir().join(format!("actus-publication-absent-{}", std::process::id()));
        let expected = ActusMetadata {
            size: 0,
            is_file: 0,
            is_dir: 0,
            readonly: 0,
            is_symlink: 0,
            modified_seconds: 0,
            modified_status: 2,
            accessed_seconds: 0,
            accessed_status: 2,
            created_seconds: 0,
            created_status: 2,
            identity_primary: 0,
            identity_secondary: 0,
            identity_kind: 0,
            identity_status: 2,
        };
        assert_eq!(compare_expected_identity(&expected, &path), Ok(true));
    }

    #[cfg(windows)]
    #[test]
    fn sync_parent_directory_reports_unsupported_target() {
        let path = std::path::Path::new("publication-target");
        assert_eq!(
            sync_parent_directory(path).expect_err("Windows directory sync is unsupported").kind(),
            std::io::ErrorKind::Unsupported
        );
    }

    #[cfg(windows)]
    #[test]
    fn replace_atomic_publishes_when_destination_exists_or_is_absent() {
        let root = std::env::temp_dir().join(format!("actus-publication-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary publication directory");
        let destination = root.join("destination");
        let staging = root.join("staging");
        std::fs::write(&destination, b"old").expect("destination write");
        std::fs::write(&staging, b"new").expect("staging write");
        super::replace_atomic(&staging, &destination).expect("existing destination replacement");
        assert_eq!(std::fs::read(&destination).expect("replacement read"), b"new");

        let absent_destination = root.join("absent-destination");
        let second_staging = root.join("second-staging");
        std::fs::write(&second_staging, b"fresh").expect("second staging write");
        super::replace_atomic(&second_staging, &absent_destination)
            .expect("absent destination publication");
        assert_eq!(std::fs::read(&absent_destination).expect("publication read"), b"fresh");

        let _ = std::fs::remove_file(destination);
        let _ = std::fs::remove_file(absent_destination);
        let _ = std::fs::remove_dir(root);
    }
}

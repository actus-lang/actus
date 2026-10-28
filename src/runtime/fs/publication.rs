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
        let _ = (from, to);
        return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
    }
    #[allow(unreachable_code)]
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
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
    let comparison = unsafe { with_host_path(destination, |path| path.metadata()) };
    let identity_matches = match compare_expected_identity(expected, comparison) {
        Ok(value) => value,
        Err(status) => {
            let _ = super::file::actus_file_unlock(i64::from(lock));
            let _ = super::file::actus_file_close(i64::from(lock));
            return status;
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
    current: Option<std::io::Result<std::fs::Metadata>>,
) -> Result<bool, i32> {
    if expected.identity_kind == 0 {
        return match current {
            Some(Ok(_)) => Ok(false),
            Some(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Some(Err(error)) if error.kind() == std::io::ErrorKind::Unsupported => {
                Err(PUBLICATION_STATUS_UNSUPPORTED)
            }
            Some(Err(_)) | None => Err(ABI_STATUS_FAILURE),
        };
    }
    #[cfg(windows)]
    if expected.identity_kind == 2 {
        return Err(PUBLICATION_STATUS_UNSUPPORTED);
    }
    let Some(current) = current else {
        return Err(ABI_STATUS_FAILURE);
    };
    let current = match current {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
            return Err(PUBLICATION_STATUS_UNSUPPORTED);
        }
        Err(_) => return Err(ABI_STATUS_FAILURE),
    };
    let current_identity = native_identity(&current);
    Ok(current_identity
        == Some((expected.identity_primary, expected.identity_secondary, expected.identity_kind)))
}

fn native_identity(metadata: &std::fs::Metadata) -> Option<(u64, u64, i32)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return Some((metadata.dev(), metadata.ino(), 1));
    }
    #[cfg(windows)]
    {
        let _ = metadata;
        return None;
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
    use super::sync_parent_directory;

    #[cfg(unix)]
    #[test]
    fn sync_parent_directory_accepts_existing_parent() {
        let path = std::env::temp_dir().join("actus-publication-test");
        sync_parent_directory(&path).expect("temporary directory should be syncable");
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
}

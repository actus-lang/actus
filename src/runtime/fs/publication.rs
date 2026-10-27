use super::super::contract::{ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS};
use super::super::path::ActusPath;
use super::path_bridge::{with_host_path, with_host_paths};

const PUBLICATION_STATUS_UNSUPPORTED: i32 = -2;

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

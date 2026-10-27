use super::super::contract::ABI_STATUS_FAILURE;
use super::super::path::ActusPath;
use super::super::types::BufferHandle;
use super::operations::{path_pair, same_file_identity};
use super::path_bridge::with_host_paths;

const COPY_STATUS_ALREADY_EXISTS: i32 = -2;

/// Copies a file and returns the number of copied bytes, or `-1` on failure.
///
/// # Safety
/// `from` and `to` must point to valid live `ActusBuffer` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_copy_buffer(from: BufferHandle, to: BufferHandle) -> i32 {
    let Some((from, to)) = (unsafe { path_pair(from, to) }) else { return ABI_STATUS_FAILURE };
    std::fs::copy(from, to)
        .map_or(ABI_STATUS_FAILURE, |count| i32::try_from(count).unwrap_or(ABI_STATUS_FAILURE))
}

/// Copies a file between two validated borrowed `ActusPath` values.
///
/// # Safety
/// Both paths must be valid immutable descriptors for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_copy_path(from: *const ActusPath, to: *const ActusPath) -> i32 {
    unsafe { actus_file_copy_options_path(from, to, 1, 0) }
}

/// Copies a file with explicit overwrite and permission-metadata policies.
///
/// # Safety
/// Both paths must be valid immutable descriptors for this call. `overwrite`
/// and `preserve_metadata` are scalar switches where zero is disabled and any
/// non-zero value is enabled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_copy_options_path(
    from: *const ActusPath,
    to: *const ActusPath,
    overwrite: i32,
    preserve_metadata: i32,
) -> i32 {
    let result = unsafe {
        with_host_paths(from, to, |source, destination| {
            copy_file_with_policy(source, destination, overwrite != 0, preserve_metadata != 0)
        })
    };
    match result {
        Some(Ok(count)) => i32::try_from(count).unwrap_or(ABI_STATUS_FAILURE),
        Some(Err(error)) => copy_error_status(&error),
        None => ABI_STATUS_FAILURE,
    }
}

fn copy_error_status(error: &std::io::Error) -> i32 {
    match error.kind() {
        std::io::ErrorKind::AlreadyExists => COPY_STATUS_ALREADY_EXISTS,
        _ => ABI_STATUS_FAILURE,
    }
}

fn copy_file_with_policy(
    source: &std::path::Path,
    destination: &std::path::Path,
    overwrite: bool,
    preserve_metadata: bool,
) -> std::io::Result<u64> {
    use std::fs::{File, OpenOptions};
    use std::io;

    let source_metadata = std::fs::metadata(source)?;
    if !source_metadata.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "source is not a regular file"));
    }
    if same_file_identity(source, destination).is_ok_and(|same| same) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source and destination are identical",
        ));
    }
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create(overwrite)
        .truncate(overwrite)
        .create_new(!overwrite)
        .open(destination)?;
    let copied = match io::copy(&mut input, &mut output) {
        Ok(copied) => copied,
        Err(error) => {
            if !overwrite {
                let _ = std::fs::remove_file(destination);
            }
            return Err(error);
        }
    };
    if preserve_metadata {
        if let Err(error) = output.set_permissions(source_metadata.permissions()) {
            if !overwrite {
                let _ = std::fs::remove_file(destination);
            }
            return Err(error);
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::{COPY_STATUS_ALREADY_EXISTS, copy_error_status, copy_file_with_policy};

    #[test]
    fn copy_preserves_already_exists_error_classification() {
        let error = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
        assert_eq!(copy_error_status(&error), COPY_STATUS_ALREADY_EXISTS);
    }

    #[test]
    fn copy_without_overwrite_rejects_existing_destination() {
        let root = std::env::temp_dir().join(format!("actus-copy-{}", std::process::id()));
        let source = root.join("source");
        let destination = root.join("destination");
        std::fs::create_dir_all(&root).expect("copy test root");
        std::fs::write(&source, b"source").expect("copy source");
        std::fs::write(&destination, b"destination").expect("copy destination");
        let result = copy_file_with_policy(&source, &destination, false, false);
        assert_eq!(
            result.expect_err("existing destination must be rejected").kind(),
            std::io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(&destination).expect("destination bytes"), b"destination");
        std::fs::remove_dir_all(root).expect("copy test cleanup");
    }
}

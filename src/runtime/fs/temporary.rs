use std::path::{Path, PathBuf};

use super::super::allocation::{actus_buffer_allocate, actus_buffer_drop};
use super::super::contract::{ABI_HANDLE_FAILURE, ABI_STATUS_FAILURE};
use super::super::path::ActusPath;
use super::super::types::BufferHandle;
use super::file::handle_to_actus;
use super::path_bridge::with_host_path;

const TEMP_INVALID_PREFIX: i32 = -2;
const TEMP_COLLISION_EXHAUSTED: i32 = -3;
const TEMP_MAX_ATTEMPTS: usize = 128;
const TEMP_RANDOM_BYTES: usize = 16;

/// Opens a uniquely named file below a validated, caller-selected directory.
///
/// # Safety
/// `root`, `prefix`, and `output` must be valid live Actus values for the
/// duration of this call. `output` must be writable path storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open_temporary_path(
    root: *const ActusPath,
    prefix: BufferHandle,
    output: *mut ActusPath,
) -> i32 {
    if output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let result =
        unsafe { with_host_path(root, |directory| open_temporary_file(directory, prefix)) };
    let Some(result) = result else { return ABI_STATUS_FAILURE };
    let Ok((file, path)) = result else { return result.err().unwrap_or(ABI_STATUS_FAILURE) };
    let handle = into_runtime_handle(file);
    if handle == ABI_HANDLE_FAILURE as i32 {
        let _ = std::fs::remove_file(path);
        return ABI_STATUS_FAILURE;
    }
    if !unsafe { replace_output_path(output, &path) } {
        let _ = super::file::actus_file_close(i64::from(handle));
        let _ = std::fs::remove_file(path);
        return ABI_STATUS_FAILURE;
    }
    handle
}

/// Creates a uniquely named directory below a validated, caller-selected root.
///
/// # Safety
/// `root`, `prefix`, and `output` must be valid live Actus values for the
/// duration of this call. `output` must be writable path storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_directory_create_temporary_path(
    root: *const ActusPath,
    prefix: BufferHandle,
    output: *mut ActusPath,
) -> i32 {
    if output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let result =
        unsafe { with_host_path(root, |directory| create_temporary_directory(directory, prefix)) };
    let Some(result) = result else { return ABI_STATUS_FAILURE };
    let Ok(path) = result else { return result.err().unwrap_or(ABI_STATUS_FAILURE) };
    if !unsafe { replace_output_path(output, &path) } {
        let _ = std::fs::remove_dir(path);
        return ABI_STATUS_FAILURE;
    }
    0
}

fn open_temporary_file(
    directory: &Path,
    prefix: BufferHandle,
) -> Result<(std::fs::File, PathBuf), i32> {
    let prefix = unsafe { prefix_bytes(prefix) }.ok_or(TEMP_INVALID_PREFIX)?;
    open_temporary_file_with_prefix(directory, &prefix)
}

fn open_temporary_file_with_prefix(
    directory: &Path,
    prefix: &[u8],
) -> Result<(std::fs::File, PathBuf), i32> {
    open_temporary_file_with_candidate(directory, prefix, candidate_path)
}

fn open_temporary_file_with_candidate(
    directory: &Path,
    prefix: &[u8],
    mut candidate: impl FnMut(&Path, &[u8]) -> Result<PathBuf, i32>,
) -> Result<(std::fs::File, PathBuf), i32> {
    validate_root(directory)?;
    for _ in 0..TEMP_MAX_ATTEMPTS {
        let path = candidate(directory, prefix)?;
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create_new(true);
        match options.open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(ABI_STATUS_FAILURE),
        }
    }
    Err(TEMP_COLLISION_EXHAUSTED)
}

fn create_temporary_directory(directory: &Path, prefix: BufferHandle) -> Result<PathBuf, i32> {
    let prefix = unsafe { prefix_bytes(prefix) }.ok_or(TEMP_INVALID_PREFIX)?;
    create_temporary_directory_with_prefix(directory, &prefix)
}

fn create_temporary_directory_with_prefix(directory: &Path, prefix: &[u8]) -> Result<PathBuf, i32> {
    create_temporary_directory_with_candidate(directory, prefix, |root, name| {
        candidate_path(root, name)
    })
}

fn create_temporary_directory_with_candidate(
    directory: &Path,
    prefix: &[u8],
    mut candidate: impl FnMut(&Path, &[u8]) -> Result<PathBuf, i32>,
) -> Result<PathBuf, i32> {
    validate_root(directory)?;
    for _ in 0..TEMP_MAX_ATTEMPTS {
        let path = candidate(directory, prefix)?;
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(ABI_STATUS_FAILURE),
        }
    }
    Err(TEMP_COLLISION_EXHAUSTED)
}

fn validate_root(directory: &Path) -> Result<(), i32> {
    let metadata = std::fs::symlink_metadata(directory).map_err(|_| ABI_STATUS_FAILURE)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ABI_STATUS_FAILURE);
    }
    Ok(())
}

fn candidate_path(directory: &Path, prefix: &[u8]) -> Result<PathBuf, i32> {
    let mut random = [0u8; TEMP_RANDOM_BYTES];
    fill_random(&mut random)?;
    let mut name = String::with_capacity(prefix.len() + 1 + TEMP_RANDOM_BYTES * 2);
    name.push_str(std::str::from_utf8(prefix).map_err(|_| TEMP_INVALID_PREFIX)?);
    name.push('-');
    for byte in random {
        name.push(char::from(b"0123456789abcdef"[(byte >> 4) as usize]));
        name.push(char::from(b"0123456789abcdef"[(byte & 0x0f) as usize]));
    }
    Ok(directory.join(name))
}

unsafe fn prefix_bytes(prefix: BufferHandle) -> Option<Vec<u8>> {
    if prefix.is_null() {
        return None;
    }
    let buffer = unsafe { &*prefix };
    if buffer.length == 0 || buffer.length > 64 || buffer.length > buffer.capacity {
        return None;
    }
    if buffer.data.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    if !bytes.iter().all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(byte)) {
        return None;
    }
    Some(bytes.to_vec())
}

fn into_runtime_handle(file: std::fs::File) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::io::IntoRawFd;
        return handle_to_actus(file.into_raw_fd() as i64);
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::IntoRawHandle;
        return handle_to_actus(file.into_raw_handle() as i64);
    }
    #[allow(unreachable_code)]
    {
        ABI_HANDLE_FAILURE as i32
    }
}

fn fill_random(bytes: &mut [u8]) -> Result<(), i32> {
    #[cfg(unix)]
    {
        use std::io::Read;
        let mut source = std::fs::File::open("/dev/urandom").map_err(|_| ABI_STATUS_FAILURE)?;
        source.read_exact(bytes).map_err(|_| ABI_STATUS_FAILURE)?;
        return Ok(());
    }
    #[cfg(windows)]
    {
        const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x00000002;
        #[link(name = "bcrypt")]
        unsafe extern "system" {
            fn BCryptGenRandom(
                algorithm: *mut std::ffi::c_void,
                buffer: *mut u8,
                length: u32,
                flags: u32,
            ) -> i32;
        }
        let length = u32::try_from(bytes.len()).map_err(|_| ABI_STATUS_FAILURE)?;
        let status = unsafe {
            BCryptGenRandom(
                std::ptr::null_mut(),
                bytes.as_mut_ptr(),
                length,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            )
        };
        if status == 0 {
            return Ok(());
        }
        return Err(ABI_STATUS_FAILURE);
    }
    #[allow(unreachable_code)]
    Err(ABI_STATUS_FAILURE)
}

unsafe fn replace_output_path(output: *mut ActusPath, path: &Path) -> bool {
    let bytes = path_bytes(path);
    let Some(mut storage) = bytes else { return false };
    storage.push(0);
    let handle = actus_buffer_allocate(storage.len());
    if handle.is_null() {
        return false;
    }
    let buffer = unsafe { &mut *handle };
    unsafe { std::ptr::copy_nonoverlapping(storage.as_ptr(), buffer.data, storage.len()) };
    let output = unsafe { &mut *output };
    if !output.storage.is_null() {
        unsafe { actus_buffer_drop(output.storage) };
    }
    output.storage = handle;
    output.platform = if cfg!(windows) { 1 } else { 0 };
    output.length = i32::try_from(storage.len() - 1).unwrap_or(-1);
    output.capacity = i32::try_from(storage.len()).unwrap_or(-1);
    output.terminated = 1;
    output.length >= 0 && output.capacity >= 0
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Option<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    Some(path.as_os_str().as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::thread;

    use super::{
        TEMP_COLLISION_EXHAUSTED, create_temporary_directory_with_candidate,
        create_temporary_directory_with_prefix, open_temporary_file_with_candidate,
    };

    fn test_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("actus-temp-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).expect("create temporary-resource test root");
        root
    }

    #[test]
    fn collision_retry_is_bounded_and_atomic() {
        let root = test_root("collision");
        let occupied = root.join("fixed-name");
        fs::write(&occupied, b"occupied").expect("create occupied candidate");
        let mut attempts = 0;
        let result =
            create_temporary_directory_with_candidate(&root, b"collision", |directory, _| {
                attempts += 1;
                if attempts == 1 {
                    return Ok(occupied.clone());
                }
                Ok(directory.join(format!("collision-{}", attempts)))
            });
        let created = result.expect("retry should create a fresh directory");
        assert_eq!(attempts, 2);
        assert!(created.is_dir());
        fs::remove_dir_all(&root).expect("remove collision test root");

        let exhausted_root = test_root("exhausted");
        let occupied = exhausted_root.join("fixed-name");
        fs::create_dir(&occupied).expect("create occupied directory");
        let result =
            create_temporary_directory_with_candidate(&exhausted_root, b"collision", |_, _| {
                Ok(occupied.clone())
            });
        assert_eq!(result, Err(TEMP_COLLISION_EXHAUSTED));
        fs::remove_dir_all(&exhausted_root).expect("remove exhausted test root");
    }

    #[test]
    fn concurrent_directory_creators_receive_distinct_paths() {
        let root = Arc::new(test_root("concurrent"));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let root = Arc::clone(&root);
            workers.push(thread::spawn(move || {
                create_temporary_directory_with_prefix(&root, b"concurrent")
                    .expect("concurrent creator should succeed")
            }));
        }
        let paths = workers
            .into_iter()
            .map(|worker| worker.join().expect("creator thread should not panic"))
            .collect::<Vec<_>>();
        let unique = paths.iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), paths.len());
        for path in paths {
            assert!(path.is_dir());
        }
        fs::remove_dir_all(&*root).expect("remove concurrent test root");
    }

    #[test]
    fn non_directory_workspace_is_rejected_before_creation() {
        let root = test_root("restricted");
        let file = root.join("not-a-directory");
        fs::write(&file, b"not a workspace").expect("create invalid workspace");
        let result = create_temporary_directory_with_prefix(&file, b"rejected");
        assert!(result.is_err());
        fs::remove_dir_all(&root).expect("remove restricted test root");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_workspace_is_rejected_before_creation() {
        use std::os::unix::fs::symlink;

        let root = test_root("symlink");
        let target = root.join("target");
        let link = root.join("link");
        fs::create_dir(&target).expect("create symlink target");
        symlink(&target, &link).expect("create workspace symlink");
        let result = create_temporary_directory_with_prefix(&link, b"rejected");
        assert!(result.is_err());
        fs::remove_dir_all(&target).expect("remove symlink target");
        fs::remove_file(&link).expect("remove workspace symlink");
        fs::remove_dir_all(&root).expect("remove symlink test root");
    }

    #[cfg(unix)]
    #[test]
    fn read_only_workspace_is_rejected_before_creation() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_root("readonly");
        let readonly = root.join("readonly");
        fs::create_dir(&readonly).expect("create readonly workspace");
        let original = fs::metadata(&readonly).expect("inspect readonly workspace").permissions();
        let mut locked = original.clone();
        locked.set_mode(0o500);
        fs::set_permissions(&readonly, locked).expect("make workspace readonly");
        let result = create_temporary_directory_with_prefix(&readonly, b"rejected");
        fs::set_permissions(&readonly, original).expect("restore workspace permissions");
        assert!(result.is_err());
        fs::remove_dir_all(&root).expect("remove readonly test root");
    }

    #[test]
    fn file_collision_retry_preserves_existing_file() {
        let root = test_root("file-collision");
        let occupied = root.join("fixed-file");
        fs::write(&occupied, b"keep").expect("create occupied file");
        let mut attempts = 0;
        let (file, created) =
            open_temporary_file_with_candidate(&root, b"file", |directory: &Path, _| {
                attempts += 1;
                if attempts == 1 {
                    return Ok(occupied.clone());
                }
                Ok(directory.join(format!("file-{}", attempts)))
            })
            .expect("file retry should create a fresh file");
        drop(file);
        assert_eq!(fs::read(&occupied).expect("read occupied file"), b"keep");
        assert!(created.is_file());
        fs::remove_dir_all(&root).expect("remove file collision test root");
    }
}

#[cfg(windows)]
fn path_bytes(path: &Path) -> Option<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    let mut bytes = Vec::new();
    for unit in path.as_os_str().encode_wide() {
        bytes.extend_from_slice(&unit.to_ne_bytes());
    }
    Some(bytes)
}

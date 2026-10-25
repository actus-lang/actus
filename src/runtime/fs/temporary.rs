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
    validate_root(directory)?;
    for _ in 0..TEMP_MAX_ATTEMPTS {
        let path = candidate_path(directory, &prefix)?;
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
    validate_root(directory)?;
    for _ in 0..TEMP_MAX_ATTEMPTS {
        let path = candidate_path(directory, &prefix)?;
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

#[cfg(windows)]
fn path_bytes(path: &Path) -> Option<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    let mut bytes = Vec::new();
    for unit in path.as_os_str().encode_wide() {
        bytes.extend_from_slice(&unit.to_ne_bytes());
    }
    Some(bytes)
}

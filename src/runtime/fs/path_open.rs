use std::path::Path;

use super::super::contract::ABI_HANDLE_FAILURE;
use super::super::path::ActusPath;
use super::path_bridge::with_host_path;
use super::{MODE_CREATE, handle_to_actus};

/// Opens a validated `ActusPath` without converting it through UTF-8.
///
/// # Safety
/// `path` must be null or a valid immutable `ActusPath` for the duration of
/// this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open_path(path: *const ActusPath, mode: i32) -> i32 {
    let handle = unsafe { with_host_path(path, |native| open_host_path(native, mode as u32)) };
    handle.map_or(ABI_HANDLE_FAILURE as i32, handle_to_actus)
}

/// Opens a validated `ActusPath` using independent scalar options.
///
/// # Safety
/// `path` must be null or a valid immutable `ActusPath` for the duration of
/// this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_file_open_options_path(
    path: *const ActusPath,
    read: i32,
    write: i32,
    append: i32,
    truncate: i32,
    create: i32,
    create_new: i32,
) -> i32 {
    let handle = unsafe {
        with_host_path(path, |native| {
            let mut options = std::fs::OpenOptions::new();
            options
                .read(read != 0)
                .write(write != 0 || append != 0)
                .append(append != 0)
                .truncate(truncate != 0)
                .create(create != 0)
                .create_new(create_new != 0);
            open_host_options(native, options)
        })
    };
    handle.map_or(ABI_HANDLE_FAILURE as i32, handle_to_actus)
}

fn open_host_path(path: &Path, mode: u32) -> i64 {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    if mode == MODE_CREATE {
        options.write(true).create(true).truncate(true);
    }
    open_host_options(path, options)
}

fn open_host_options(path: &Path, options: std::fs::OpenOptions) -> i64 {
    let Ok(file) = options.open(path) else { return ABI_HANDLE_FAILURE };
    #[cfg(unix)]
    {
        use std::os::unix::io::IntoRawFd;
        return file.into_raw_fd() as i64;
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::IntoRawHandle;
        return file.into_raw_handle() as i64;
    }
    #[allow(unreachable_code)]
    ABI_HANDLE_FAILURE
}

use std::path::Path;

use super::super::allocation::{actus_buffer_allocate, actus_buffer_drop};
use super::super::path::ActusPath;
#[cfg(unix)]
use super::super::path::{ActusPathCView, actus_path_c_view};
#[cfg(windows)]
use super::super::path::{ActusPathWideCView, actus_path_wide_c_view};

/// Runs a borrowed host path operation without retaining the source pointer.
/// POSIX uses a borrowed byte slice; Windows constructs the host wide path
/// from validated code units for the duration of the call.
pub(crate) unsafe fn with_host_path<T>(
    path: *const ActusPath,
    operation: impl FnOnce(&Path) -> T,
) -> Option<T> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let mut view = ActusPathCView { data: std::ptr::null(), length: 0 };
        if unsafe { actus_path_c_view(&mut view, path) }.is_null() {
            return None;
        }
        let bytes = unsafe { std::slice::from_raw_parts(view.data, view.length as usize) };
        return Some(operation(Path::new(std::ffi::OsStr::from_bytes(bytes))));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let mut view = ActusPathWideCView { data: std::ptr::null(), length: 0 };
        if unsafe { actus_path_wide_c_view(&mut view, path) }.is_null() {
            return None;
        }
        let units = unsafe { std::slice::from_raw_parts(view.data, view.length as usize) };
        let native = std::ffi::OsString::from_wide(units);
        return Some(operation(Path::new(&native)));
    }
    #[allow(unreachable_code)]
    None
}

/// Replaces an owned Actus path only after the host operation produced a path.
pub(crate) unsafe fn replace_output_path(output: *mut ActusPath, path: &Path) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(mut storage) = path_bytes(path) else { return false };
    #[cfg(unix)]
    storage.push(0);
    #[cfg(windows)]
    storage.extend_from_slice(&0u16.to_ne_bytes());
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
    let length = if cfg!(windows) {
        storage.len().saturating_div(2).saturating_sub(1)
    } else {
        storage.len().saturating_sub(1)
    };
    output.length = i32::try_from(length).unwrap_or(-1);
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

/// Runs one operation with two borrowed host paths and rejects invalid views.
pub(crate) unsafe fn with_host_paths<T>(
    from: *const ActusPath,
    to: *const ActusPath,
    operation: impl FnOnce(&Path, &Path) -> T,
) -> Option<T> {
    #[cfg(unix)]
    return unsafe { with_unix_host_paths(from, to, operation) };
    #[cfg(windows)]
    return unsafe { with_windows_host_paths(from, to, operation) };
    #[allow(unreachable_code)]
    None
}

#[cfg(unix)]
unsafe fn with_unix_host_paths<T>(
    from: *const ActusPath,
    to: *const ActusPath,
    operation: impl FnOnce(&Path, &Path) -> T,
) -> Option<T> {
    use std::os::unix::ffi::OsStrExt;
    let mut source = ActusPathCView { data: std::ptr::null(), length: 0 };
    let mut destination = ActusPathCView { data: std::ptr::null(), length: 0 };
    if unsafe { actus_path_c_view(&mut source, from) }.is_null()
        || unsafe { actus_path_c_view(&mut destination, to) }.is_null()
    {
        return None;
    }
    let source_bytes = unsafe { std::slice::from_raw_parts(source.data, source.length as usize) };
    let destination_bytes =
        unsafe { std::slice::from_raw_parts(destination.data, destination.length as usize) };
    Some(operation(
        Path::new(std::ffi::OsStr::from_bytes(source_bytes)),
        Path::new(std::ffi::OsStr::from_bytes(destination_bytes)),
    ))
}

#[cfg(windows)]
unsafe fn with_windows_host_paths<T>(
    from: *const ActusPath,
    to: *const ActusPath,
    operation: impl FnOnce(&Path, &Path) -> T,
) -> Option<T> {
    use std::os::windows::ffi::OsStringExt;
    let mut source = ActusPathWideCView { data: std::ptr::null(), length: 0 };
    let mut destination = ActusPathWideCView { data: std::ptr::null(), length: 0 };
    if unsafe { actus_path_wide_c_view(&mut source, from) }.is_null()
        || unsafe { actus_path_wide_c_view(&mut destination, to) }.is_null()
    {
        return None;
    }
    let source_units = unsafe { std::slice::from_raw_parts(source.data, source.length as usize) };
    let destination_units =
        unsafe { std::slice::from_raw_parts(destination.data, destination.length as usize) };
    let source_path = std::ffi::OsString::from_wide(source_units);
    let destination_path = std::ffi::OsString::from_wide(destination_units);
    Some(operation(Path::new(&source_path), Path::new(&destination_path)))
}

use std::path::Path;

use super::path::ActusPath;
#[cfg(unix)]
use super::path_c_view::{ActusPathCView, actus_path_c_view};
#[cfg(windows)]
use super::path_c_view::{ActusPathWideCView, actus_path_wide_c_view};

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

/// Runs one operation with two borrowed host paths and rejects invalid views.
pub(crate) unsafe fn with_host_paths<T>(
    from: *const ActusPath,
    to: *const ActusPath,
    operation: impl FnOnce(&Path, &Path) -> T,
) -> Option<T> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let mut source = ActusPathCView { data: std::ptr::null(), length: 0 };
        let mut destination = ActusPathCView { data: std::ptr::null(), length: 0 };
        if unsafe { actus_path_c_view(&mut source, from) }.is_null()
            || unsafe { actus_path_c_view(&mut destination, to) }.is_null()
        {
            return None;
        }
        let source_bytes =
            unsafe { std::slice::from_raw_parts(source.data, source.length as usize) };
        let destination_bytes =
            unsafe { std::slice::from_raw_parts(destination.data, destination.length as usize) };
        return Some(operation(
            Path::new(std::ffi::OsStr::from_bytes(source_bytes)),
            Path::new(std::ffi::OsStr::from_bytes(destination_bytes)),
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let mut source = ActusPathWideCView { data: std::ptr::null(), length: 0 };
        let mut destination = ActusPathWideCView { data: std::ptr::null(), length: 0 };
        if unsafe { actus_path_wide_c_view(&mut source, from) }.is_null()
            || unsafe { actus_path_wide_c_view(&mut destination, to) }.is_null()
        {
            return None;
        }
        let source_units =
            unsafe { std::slice::from_raw_parts(source.data, source.length as usize) };
        let destination_units =
            unsafe { std::slice::from_raw_parts(destination.data, destination.length as usize) };
        let source_path = std::ffi::OsString::from_wide(source_units);
        let destination_path = std::ffi::OsString::from_wide(destination_units);
        return Some(operation(Path::new(&source_path), Path::new(&destination_path)));
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(windows)]
#[repr(C)]
struct WindowsFileInformation {
    attributes: u32,
    creation_time_low: u32,
    creation_time_high: u32,
    last_access_time_low: u32,
    last_access_time_high: u32,
    last_write_time_low: u32,
    last_write_time_high: u32,
    volume_serial_number: u32,
    file_size_high: u32,
    file_size_low: u32,
    number_of_links: u32,
    file_index_high: u32,
    file_index_low: u32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut std::ffi::c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    fn GetFileInformationByHandle(
        handle: *mut std::ffi::c_void,
        information: *mut WindowsFileInformation,
    ) -> i32;
}

#[cfg(windows)]
const INVALID_HANDLE_VALUE: *mut std::ffi::c_void = -1isize as *mut std::ffi::c_void;
#[cfg(windows)]
const FILE_SHARE_ALL: u32 = 0x0000_0007;
#[cfg(windows)]
const OPEN_EXISTING: u32 = 3;
#[cfg(windows)]
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

#[cfg(windows)]
pub(super) fn windows_file_identity(path: &std::path::Path) -> Result<(u64, u64), ()> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    wide.push(0);
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            0,
            FILE_SHARE_ALL,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(());
    }
    let identity = windows_file_identity_handle(handle);
    let closed = unsafe { CloseHandle(handle) } != 0;
    if !closed {
        return Err(());
    }
    identity
}

#[cfg(windows)]
pub(super) fn windows_file_identity_handle(
    handle: *mut std::ffi::c_void,
) -> Result<(u64, u64), ()> {
    let mut information: WindowsFileInformation = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(handle, &mut information) } == 0 {
        return Err(());
    }
    let volume = u64::from(information.volume_serial_number);
    let index =
        (u64::from(information.file_index_high) << 32) | u64::from(information.file_index_low);
    Ok((volume, index))
}

#[cfg(all(test, windows))]
mod tests {
    use super::windows_file_identity;

    #[test]
    fn windows_identity_is_stable_for_one_path_and_distinct_for_two_files() {
        let root = std::env::temp_dir().join(format!("actus-identity-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary identity directory");
        let first = root.join("first");
        let second = root.join("second");
        std::fs::write(&first, b"first").expect("first file");
        std::fs::write(&second, b"second").expect("second file");

        let first_identity = windows_file_identity(&first).expect("first identity");
        assert_eq!(windows_file_identity(&first).expect("stable first identity"), first_identity);
        assert_ne!(first_identity, windows_file_identity(&second).expect("second identity"));

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
        let _ = std::fs::remove_dir(root);
    }
}

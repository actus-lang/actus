use actus::runtime::{
    ActusBuffer, ActusPath, PathErrorCode, actus_path_join, actus_path_push, actus_path_reserve,
    actus_path_set_extension, actus_path_set_file_name,
};

fn raw_path(bytes: &[u8], platform: i32) -> *mut ActusPath {
    let mut storage = std::mem::ManuallyDrop::new(bytes.to_vec());
    let buffer = Box::into_raw(Box::new(ActusBuffer {
        data: storage.as_mut_ptr(),
        length: storage.len(),
        capacity: storage.capacity(),
    }));
    Box::into_raw(Box::new(ActusPath {
        storage: buffer,
        platform,
        length: if platform == 0 {
            (storage.len() - 1) as i32
        } else {
            ((storage.len() - 2) / 2) as i32
        },
        capacity: storage.capacity() as i32,
        terminated: 1,
    }))
}

fn path_bytes(path: *const ActusPath) -> Vec<u8> {
    unsafe {
        let path = &*path;
        let buffer = &*path.storage;
        std::slice::from_raw_parts(buffer.data, buffer.length).to_vec()
    }
}

fn utf16(units: &[u16]) -> Vec<u8> {
    units.iter().flat_map(|unit| unit.to_ne_bytes()).collect()
}

#[test]
fn builders_join_and_mutate_posix_storage_in_place() {
    let left = raw_path(b"/tmp/archive.tar\0", 0);
    let right = raw_path(b"logs\0", 0);
    assert_eq!(unsafe { actus_path_join(left, right) }, 0);
    assert_eq!(path_bytes(left), b"/tmp/archive.tar/logs\0");

    let extension = raw_path(b"gz\0", 0);
    assert_eq!(unsafe { actus_path_set_extension(left, extension) }, 0);
    assert_eq!(path_bytes(left), b"/tmp/archive.tar/logs.gz\0");

    let name = raw_path(b"final.bin\0", 0);
    assert_eq!(unsafe { actus_path_set_file_name(left, name) }, 0);
    assert_eq!(path_bytes(left), b"/tmp/archive.tar/final.bin\0");
}

#[test]
fn push_and_reserve_preserve_windows_code_units() {
    let left = raw_path(&utf16(&[b'C' as u16, b':'.into(), b'\\' as u16, b'a' as u16, 0]), 1);
    let right = raw_path(&utf16(&[b'b' as u16, 0]), 1);
    let old_capacity = unsafe { (*left).capacity };
    assert_eq!(unsafe { actus_path_reserve(left, old_capacity + 16) }, 0);
    assert!(unsafe { (*left).capacity } >= old_capacity + 16);
    assert_eq!(unsafe { actus_path_push(left, right) }, 0);
    assert_eq!(
        path_bytes(left),
        utf16(&[b'C' as u16, b':'.into(), b'\\' as u16, b'a' as u16, b'\\' as u16, b'b' as u16, 0])
    );
}

#[test]
fn builders_reject_incompatible_platforms_and_path_separators() {
    let left = raw_path(b"base\0", 0);
    let windows = raw_path(&utf16(&[b'x' as u16, 0]), 1);
    assert_eq!(
        unsafe { actus_path_push(left, windows) },
        PathErrorCode::UnsupportedPlatform as i32
    );

    let invalid_name = raw_path(b"bad/name\0", 0);
    assert_eq!(
        unsafe { actus_path_set_file_name(left, invalid_name) },
        PathErrorCode::InvalidLength as i32
    );
    let invalid_extension = raw_path(b"bad.ext\0", 0);
    assert_eq!(unsafe { actus_path_set_extension(left, invalid_extension) }, 0);
}

#[test]
fn extension_replacement_after_file_name_replacement_preserves_storage() {
    let left = raw_path(b"a/b/c/c\0", 0);
    let name = raw_path(b"final\0", 0);
    let extension = raw_path(b"log\0", 0);
    assert_eq!(unsafe { actus_path_set_file_name(left, name) }, 0);
    assert_eq!(unsafe { actus_path_set_extension(left, extension) }, 0);
    assert_eq!(path_bytes(left), b"a/b/c/final.log\0");
}

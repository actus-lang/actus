use actus::runtime::{
    ActusBuffer, ActusPath, ActusPathCView, ActusPathWideCView, actus_path_c_view,
    actus_path_wide_c_view,
};

fn bytes_buffer(bytes: &[u8]) -> (Vec<u8>, ActusBuffer) {
    let mut storage = bytes.to_vec();
    let buffer = ActusBuffer {
        data: storage.as_mut_ptr(),
        length: storage.len(),
        capacity: storage.capacity(),
    };
    (storage, buffer)
}

fn utf16_buffer(units: &[u16]) -> (Vec<u8>, ActusBuffer) {
    let mut storage = units.iter().flat_map(|unit| unit.to_ne_bytes()).collect::<Vec<_>>();
    let buffer = ActusBuffer {
        data: storage.as_mut_ptr(),
        length: storage.len(),
        capacity: storage.capacity(),
    };
    (storage, buffer)
}

#[test]
fn posix_c_view_borrows_terminated_raw_bytes_without_allocation() {
    let (storage, mut buffer) = bytes_buffer(b"/tmp/\xff\0");
    let path = ActusPath {
        storage: &mut buffer,
        platform: 0,
        length: 6,
        capacity: storage.capacity() as i32,
        terminated: 1,
    };
    let mut view = ActusPathCView { data: std::ptr::null(), length: 0 };
    assert!(std::ptr::eq(unsafe { actus_path_c_view(&mut view, &path) }, &view));
    assert_eq!(view.length, 6);
    assert!(std::ptr::eq(view.data, storage.as_ptr()));
    assert_eq!(
        unsafe { std::slice::from_raw_parts(view.data, view.length as usize + 1) },
        b"/tmp/\xff\0"
    );
}

#[test]
fn windows_c_view_borrows_native_utf16_units_and_terminator() {
    let units = [b'C' as u16, b':'.into(), b'\\' as u16, 0x03A9, 0];
    let (storage, mut buffer) = utf16_buffer(&units);
    let path = ActusPath {
        storage: &mut buffer,
        platform: 1,
        length: 4,
        capacity: storage.capacity() as i32,
        terminated: 1,
    };
    let mut view = ActusPathWideCView { data: std::ptr::null(), length: 0 };
    assert!(std::ptr::eq(unsafe { actus_path_wide_c_view(&mut view, &path) }, &view));
    assert_eq!(view.length, 4);
    assert_eq!(unsafe { std::slice::from_raw_parts(view.data, view.length as usize + 1) }, units);
}

#[test]
fn c_views_reject_wrong_platform_and_malformed_termination() {
    let (mut storage, mut buffer) = bytes_buffer(b"plain\0");
    let path = ActusPath {
        storage: &mut buffer,
        platform: 0,
        length: 5,
        capacity: storage.capacity() as i32,
        terminated: 1,
    };
    let mut byte_view = ActusPathCView { data: std::ptr::null(), length: 0 };
    let mut wide_view = ActusPathWideCView { data: std::ptr::null(), length: 0 };
    assert!(unsafe { actus_path_wide_c_view(&mut wide_view, &path) }.is_null());
    assert!(unsafe { actus_path_c_view(std::ptr::null_mut(), &path) }.is_null());
    storage[5] = b'x';
    assert!(unsafe { actus_path_c_view(&mut byte_view, &path) }.is_null());
}

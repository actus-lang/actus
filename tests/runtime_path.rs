use actus::runtime::{
    ActusBuffer, ActusPath, PathErrorCode, PosixRoot, WindowsRoot, actus_posix_is_separator,
    actus_posix_root_kind, actus_windows_is_separator, actus_windows_root_kind, components,
    extension, file_name, file_stem, posix_root, validate_posix, validate_windows, windows_root,
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

fn units_buffer(units: &[u16]) -> (Vec<u8>, ActusBuffer) {
    let mut storage = units.iter().flat_map(|unit| unit.to_ne_bytes()).collect::<Vec<_>>();
    let buffer = ActusBuffer {
        data: storage.as_mut_ptr(),
        length: storage.len(),
        capacity: storage.capacity(),
    };
    (storage, buffer)
}

#[test]
fn posix_runtime_preserves_non_utf8_bytes_and_root_rules() {
    assert_eq!(posix_root(b"///usr//\xff/bin"), Ok(PosixRoot::Absolute));
    assert_eq!(posix_root(b"foo/bar"), Ok(PosixRoot::Relative));
    assert_eq!(validate_posix(b".\0"), Err(PathErrorCode::EmbeddedNull));
}

#[test]
fn windows_runtime_distinguishes_drive_and_unc_roots() {
    assert_eq!(
        windows_root(&[b'C' as u16, b':'.into(), b'f' as u16]),
        Ok(WindowsRoot::DriveRelative)
    );
    assert_eq!(
        windows_root(&[b'C' as u16, b':'.into(), b'\\' as u16]),
        Ok(WindowsRoot::DriveAbsolute)
    );
    assert_eq!(windows_root(&[b'\\' as u16, b'\\' as u16, b's' as u16]), Ok(WindowsRoot::Unc));
    assert_eq!(validate_windows(&[0xD800]), Err(PathErrorCode::InvalidEncoding));
}

#[test]
fn component_iterator_is_borrowed_and_platform_aware() {
    let units = b"a\\b/c".iter().map(|byte| u16::from(*byte)).collect::<Vec<_>>();
    let ranges = components(&units, true).collect::<Vec<_>>();
    assert_eq!(
        ranges.iter().map(|range| &units[range.start..range.end]).collect::<Vec<_>>(),
        [&[b'a' as u16][..], &[b'b' as u16][..], &[b'c' as u16][..]]
    );
    let archive = b"archive.tar.gz".iter().map(|byte| u16::from(*byte)).collect::<Vec<_>>();
    assert_eq!(
        file_name(&archive, false),
        Some(0..14)
            .map(|range| actus::runtime::ComponentRange { start: range.start, end: range.end })
    );
    assert_eq!(
        file_stem(&archive, false),
        Some(actus::runtime::ComponentRange { start: 0, end: 11 })
    );
    assert_eq!(
        extension(&archive, false),
        Some(actus::runtime::ComponentRange { start: 12, end: 14 })
    );
    assert_eq!(actus_posix_is_separator(47), 1);
    assert_eq!(actus_windows_is_separator(92), 1);
}

#[test]
fn c_abi_root_classification_reads_the_real_raw_buffer() {
    let (mut storage, mut buffer) = bytes_buffer(b"/tmp\0");
    let path = ActusPath {
        storage: &mut buffer,
        platform: 0,
        length: 4,
        capacity: storage.capacity() as i32,
        terminated: 1,
    };
    assert_eq!(unsafe { actus_posix_root_kind(&path) }, 1);
    storage[0] = b'x';
    assert_eq!(unsafe { actus_posix_root_kind(&path) }, 0);

    let (storage, mut buffer) = units_buffer(&[b'C' as u16, b':'.into(), b'\\' as u16, 0]);
    let path = ActusPath {
        storage: &mut buffer,
        platform: 1,
        length: 3,
        capacity: storage.capacity() as i32,
        terminated: 1,
    };
    assert_eq!(unsafe { actus_windows_root_kind(&path) }, 2);
}

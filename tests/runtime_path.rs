use actus::runtime::{
    ActusBuffer, ActusPath, PathErrorCode, PosixRoot, WindowsRoot, actus_path_ends_with,
    actus_path_has_root, actus_path_is_absolute, actus_path_is_relative, actus_path_normalize,
    actus_path_starts_with, actus_posix_is_separator, actus_posix_root_kind,
    actus_windows_is_separator, actus_windows_root_kind, components, extension, file_name,
    file_stem, posix_root, validate_posix, validate_windows, windows_root,
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

#[test]
fn predicates_compare_complete_components_and_roots() {
    let (mut path_storage, mut path_buffer) = bytes_buffer(b"/usr/bin\0");
    let path = ActusPath {
        storage: &mut path_buffer,
        platform: 0,
        length: 8,
        capacity: path_storage.capacity() as i32,
        terminated: 1,
    };
    let (mut prefix_storage, mut prefix_buffer) = bytes_buffer(b"/usr\0");
    let prefix = ActusPath {
        storage: &mut prefix_buffer,
        platform: 0,
        length: 4,
        capacity: prefix_storage.capacity() as i32,
        terminated: 1,
    };
    let (mut partial_storage, mut partial_buffer) = bytes_buffer(b"/us\0");
    let partial = ActusPath {
        storage: &mut partial_buffer,
        platform: 0,
        length: 3,
        capacity: partial_storage.capacity() as i32,
        terminated: 1,
    };
    assert_eq!(unsafe { actus_path_is_absolute(&path) }, 1);
    assert_eq!(unsafe { actus_path_is_relative(&path) }, 0);
    assert_eq!(unsafe { actus_path_has_root(&path) }, 1);
    assert_eq!(unsafe { actus_path_starts_with(&path, &prefix) }, 1);
    assert_eq!(unsafe { actus_path_starts_with(&path, &partial) }, 0);
    assert_eq!(unsafe { actus_path_ends_with(&path, &prefix) }, 0);
    let _ = (&mut path_storage, &mut prefix_storage, &mut partial_storage);
}

#[test]
fn posix_normalization_is_lexical_and_stays_inside_root() {
    for (source, expected) in [
        (&b"a/./b\0"[..], &b"a/b\0"[..]),
        (&b"a/../b\0"[..], &b"b\0"[..]),
        (&b"/a/../../b\0"[..], &b"/b\0"[..]),
        (&b"a//b\0"[..], &b"a/b\0"[..]),
        (&b".\0"[..], &b"\0"[..]),
    ] {
        let mut storage = source.to_vec();
        let capacity = storage.capacity();
        let mut buffer =
            ActusBuffer { data: storage.as_mut_ptr(), length: storage.len(), capacity };
        let mut path = ActusPath {
            storage: &mut buffer,
            platform: 0,
            length: (source.len() - 1) as i32,
            capacity: capacity as i32,
            terminated: 1,
        };
        assert_eq!(unsafe { actus_path_normalize(&mut path) }, 0);
        assert_eq!(&storage[..expected.len()], expected);
    }
}

#[test]
fn windows_normalization_preserves_drive_forms() {
    let cases = [
        (
            vec![
                b'C' as u16,
                b':'.into(),
                b'\\' as u16,
                b'a' as u16,
                b'\\' as u16,
                b'.' as u16,
                b'\\' as u16,
                b'b' as u16,
                0,
            ],
            vec![b'C' as u16, b':'.into(), b'\\' as u16, b'a' as u16, b'\\' as u16, b'b' as u16, 0],
        ),
        (
            vec![
                b'C' as u16,
                b':'.into(),
                b'\\' as u16,
                b'a' as u16,
                b'\\' as u16,
                b'.' as u16,
                b'.' as u16,
                b'\\' as u16,
                b'b' as u16,
                0,
            ],
            vec![b'C' as u16, b':'.into(), b'\\' as u16, b'b' as u16, 0],
        ),
        (
            vec![b'C' as u16, b'f' as u16, b'o' as u16, b'o' as u16, 0],
            vec![b'C' as u16, b'f' as u16, b'o' as u16, b'o' as u16, 0],
        ),
    ];
    for (source, expected) in cases {
        let mut storage = source.iter().flat_map(|unit| unit.to_ne_bytes()).collect::<Vec<_>>();
        let capacity = storage.capacity();
        let mut buffer =
            ActusBuffer { data: storage.as_mut_ptr(), length: storage.len(), capacity };
        let mut path = ActusPath {
            storage: &mut buffer,
            platform: 1,
            length: (source.len() - 1) as i32,
            capacity: capacity as i32,
            terminated: 1,
        };
        assert_eq!(unsafe { actus_path_normalize(&mut path) }, 0);
        let (pairs, _) = storage[..expected.len() * 2].as_chunks::<2>();
        let actual = pairs.iter().map(|pair| u16::from_ne_bytes(*pair)).collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}

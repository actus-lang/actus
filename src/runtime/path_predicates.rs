use super::path::{ActusPath, path_length, path_units};

#[unsafe(no_mangle)]
/// Tests whether a borrowed path has an absolute root.
///
/// # Safety
/// `path` must be null or a valid `ActusPath` with live storage.
pub unsafe extern "C" fn actus_path_is_absolute(path: *const ActusPath) -> i32 {
    let Ok((_, platform)) = (unsafe { path_units(path) }) else { return -1 };
    let kind = if platform == 0 {
        unsafe { super::path::actus_posix_root_kind(path) }
    } else {
        unsafe { super::path::actus_windows_root_kind(path) }
    };
    let absolute = if platform == 0 { kind == 1 } else { kind == 2 || kind == 3 };
    i32::from(absolute)
}

#[unsafe(no_mangle)]
/// Tests whether a borrowed path is relative.
///
/// # Safety
/// `path` must be null or a valid `ActusPath` with live storage.
pub unsafe extern "C" fn actus_path_is_relative(path: *const ActusPath) -> i32 {
    let absolute = unsafe { actus_path_is_absolute(path) };
    if absolute < 0 { -1 } else { i32::from(absolute == 0) }
}

#[unsafe(no_mangle)]
/// Tests whether a borrowed path has any platform root.
///
/// # Safety
/// `path` must be null or a valid `ActusPath` with live storage.
pub unsafe extern "C" fn actus_path_has_root(path: *const ActusPath) -> i32 {
    let Ok((_, platform)) = (unsafe { path_units(path) }) else { return -1 };
    let kind = if platform == 0 {
        unsafe { super::path::actus_posix_root_kind(path) }
    } else {
        unsafe { super::path::actus_windows_root_kind(path) }
    };
    i32::from(kind > 0)
}

#[unsafe(no_mangle)]
/// Compares complete leading components of two borrowed paths.
///
/// # Safety
/// Both pointers must be null or valid `ActusPath` values with live storage.
pub unsafe extern "C" fn actus_path_starts_with(
    path: *const ActusPath,
    prefix: *const ActusPath,
) -> i32 {
    unsafe { compare_components(path, prefix, false) }
}

#[unsafe(no_mangle)]
/// Compares complete trailing components of two borrowed paths.
///
/// # Safety
/// Both pointers must be null or valid `ActusPath` values with live storage.
pub unsafe extern "C" fn actus_path_ends_with(
    path: *const ActusPath,
    suffix: *const ActusPath,
) -> i32 {
    unsafe { compare_components(path, suffix, true) }
}

unsafe fn compare_components(path: *const ActusPath, other: *const ActusPath, ending: bool) -> i32 {
    let Ok((path_buffer, path_platform)) = (unsafe { path_units(path) }) else { return -1 };
    let Ok((other_buffer, other_platform)) = (unsafe { path_units(other) }) else { return -1 };
    if path_platform != other_platform {
        return 0;
    }
    let width = if path_platform == 0 { 1 } else { 2 };
    let path_bytes =
        unsafe { std::slice::from_raw_parts(path_buffer.data, path_length(path, width)) };
    let other_bytes =
        unsafe { std::slice::from_raw_parts(other_buffer.data, path_length(other, width)) };
    if ending {
        i32::from(components_end_with(path_bytes, other_bytes, width, path_platform == 1))
    } else {
        i32::from(components_start_with(path_bytes, other_bytes, width, path_platform == 1))
    }
}

fn components_start_with(path: &[u8], prefix: &[u8], width: usize, windows: bool) -> bool {
    let mut path_cursor = 0;
    let mut prefix_cursor = 0;
    loop {
        let path_component = next_component(path, &mut path_cursor, width, windows);
        let prefix_component = next_component(prefix, &mut prefix_cursor, width, windows);
        match (path_component, prefix_component) {
            (_, None) => return true,
            (Some(left), Some(right)) if equal_component(path, left, prefix, right, width) => {}
            _ => return false,
        }
    }
}

fn components_end_with(path: &[u8], suffix: &[u8], width: usize, windows: bool) -> bool {
    let path_count = component_count(path, width, windows);
    let suffix_count = component_count(suffix, width, windows);
    if suffix_count > path_count {
        return false;
    }
    let mut path_cursor = 0;
    let mut skipped = 0;
    while skipped < path_count - suffix_count {
        next_component(path, &mut path_cursor, width, windows);
        skipped += 1;
    }
    let mut suffix_cursor = 0;
    loop {
        let left = next_component(path, &mut path_cursor, width, windows);
        let right = next_component(suffix, &mut suffix_cursor, width, windows);
        match (left, right) {
            (None, None) => return true,
            (Some(left), Some(right)) if equal_component(path, left, suffix, right, width) => {}
            _ => return false,
        }
    }
}

fn component_count(bytes: &[u8], width: usize, windows: bool) -> usize {
    let mut cursor = 0;
    let mut count = 0;
    while next_component(bytes, &mut cursor, width, windows).is_some() {
        count += 1;
    }
    count
}

fn next_component(
    bytes: &[u8],
    cursor: &mut usize,
    width: usize,
    windows: bool,
) -> Option<(usize, usize)> {
    while *cursor + width <= bytes.len() && is_separator(read_unit(bytes, *cursor, width), windows)
    {
        *cursor += width;
    }
    let start = *cursor;
    while *cursor + width <= bytes.len() && !is_separator(read_unit(bytes, *cursor, width), windows)
    {
        *cursor += width;
    }
    (start < *cursor).then_some((start, *cursor))
}

fn equal_component(
    left: &[u8],
    left_range: (usize, usize),
    right: &[u8],
    right_range: (usize, usize),
    width: usize,
) -> bool {
    left_range.1 - left_range.0 == right_range.1 - right_range.0
        && left[left_range.0..left_range.1] == right[right_range.0..right_range.1]
        && left_range.0 % width == right_range.0 % width
}

fn read_unit(bytes: &[u8], offset: usize, width: usize) -> u16 {
    if width == 2 {
        u16::from_ne_bytes([bytes[offset], bytes[offset + 1]])
    } else {
        bytes[offset] as u16
    }
}

fn is_separator(unit: u16, windows: bool) -> bool {
    unit == b'/' as u16 || (windows && unit == b'\\' as u16)
}

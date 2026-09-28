use super::path::{ActusPath, PathErrorCode, path_units};

#[unsafe(no_mangle)]
/// Normalizes a borrowed C-layout path in its caller-owned storage.
///
/// # Safety
/// `path` must be null or a valid mutable `ActusPath` with live storage.
pub unsafe extern "C" fn actus_path_normalize(path: *mut ActusPath) -> i32 {
    let path_const = path.cast_const();
    let Ok((buffer, platform)) = (unsafe { path_units(path_const) }) else { return -4 };
    let width = if platform == 0 { 1 } else { 2 };
    let payload_length = unsafe { (*path).length as usize * width };
    let bytes = unsafe { std::slice::from_raw_parts_mut(buffer.data, payload_length) };
    let absolute = if platform == 0 {
        unsafe { super::path::actus_posix_root_kind(path_const) == 1 }
    } else {
        is_windows_absolute(path_const)
    };
    let written = if platform == 0 {
        normalize_units(bytes, 1, false, absolute)
    } else {
        normalize_units(bytes, 2, true, absolute)
    };
    let Ok(written) = written else { return written_error(written) };
    unsafe {
        if platform == 0 {
            *buffer.data.add(written) = 0;
        } else {
            let terminator = written;
            *buffer.data.add(terminator) = 0;
            *buffer.data.add(terminator + 1) = 0;
        }
        (*path).length = (written / width) as i32;
    }
    0
}

fn written_error(error: Result<usize, PathErrorCode>) -> i32 {
    error.err().map_or(-4, |error| error as i32)
}

fn is_windows_absolute(path: *const ActusPath) -> bool {
    unsafe {
        super::path::actus_windows_root_kind(path) == 2
            || super::path::actus_windows_root_kind(path) == 3
    }
}

fn normalize_units(
    bytes: &mut [u8],
    width: usize,
    windows: bool,
    absolute: bool,
) -> Result<usize, PathErrorCode> {
    let mut input_cursor = 0;
    let mut output = 0;
    let root_prefix = root_prefix(bytes, width, windows, absolute);
    if root_prefix > 0 {
        output = root_prefix;
        input_cursor = root_prefix;
    }
    while let Some((start, end)) = next_component(bytes, &mut input_cursor, width, windows) {
        let units = end - start;
        if is_dot(&bytes[start..end], width) {
            continue;
        }
        if is_dot_dot(&bytes[start..end], width) {
            if remove_previous(bytes, &mut output, width, root_prefix, windows) {
                continue;
            }
            if absolute {
                continue;
            }
        }
        if output > root_prefix {
            write_separator(bytes, &mut output, width, windows);
        }
        bytes.copy_within(start..end, output);
        output += units;
    }
    Ok(output)
}

fn root_prefix(bytes: &[u8], width: usize, windows: bool, absolute: bool) -> usize {
    if !absolute {
        return 0;
    }
    if !windows {
        return width;
    }
    if bytes.len() >= width * 3
        && is_drive(bytes, width)
        && is_separator(read_unit(bytes, width * 2, width), true)
    {
        return width * 3;
    }
    width * 2
}

fn remove_previous(
    bytes: &mut [u8],
    output: &mut usize,
    width: usize,
    root_prefix: usize,
    windows: bool,
) -> bool {
    if *output <= root_prefix {
        return false;
    }
    let mut cursor = *output;
    while cursor > root_prefix && !is_separator(read_unit(bytes, cursor - width, width), windows) {
        cursor -= width;
    }
    while cursor > root_prefix && is_separator(read_unit(bytes, cursor - width, width), windows) {
        cursor -= width;
    }
    *output = cursor;
    true
}

fn write_separator(bytes: &mut [u8], output: &mut usize, width: usize, windows: bool) {
    if width == 1 {
        bytes[*output] = b'/';
    } else {
        let value = if windows { b'\\' as u16 } else { b'/' as u16 };
        bytes[*output..*output + 2].copy_from_slice(&value.to_ne_bytes());
    }
    *output += width;
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

fn is_dot(bytes: &[u8], width: usize) -> bool {
    bytes.len() == width && read_unit(bytes, 0, width) == b'.' as u16
}

fn is_dot_dot(bytes: &[u8], width: usize) -> bool {
    bytes.len() == width * 2
        && read_unit(bytes, 0, width) == b'.' as u16
        && read_unit(bytes, width, width) == b'.' as u16
}

fn is_drive(bytes: &[u8], width: usize) -> bool {
    let unit = read_unit(bytes, 0, width);
    (b'a' as u16..=b'z' as u16).contains(&unit) || (b'A' as u16..=b'Z' as u16).contains(&unit)
}

fn is_separator(unit: u16, windows: bool) -> bool {
    unit == b'/' as u16 || (windows && unit == b'\\' as u16)
}

fn read_unit(bytes: &[u8], offset: usize, width: usize) -> u16 {
    if width == 2 {
        u16::from_ne_bytes([bytes[offset], bytes[offset + 1]])
    } else {
        bytes[offset] as u16
    }
}

use super::path::{ActusPath, PathErrorCode, path_units, validate_windows_units};
use super::types::{ActusBuffer, BufferHandle, restore_buffer};

const POSIX: i32 = 0;
const WINDOWS: i32 = 1;

fn failure(error: PathErrorCode) -> i32 {
    error as i32
}

unsafe fn payload(path: *const ActusPath) -> Result<(&'static [u8], i32), i32> {
    let (buffer, platform) = unsafe { path_units(path) }?;
    let width = if platform == POSIX { 1 } else { 2 };
    let byte_length = unsafe { (*path).length as usize } * width;
    if buffer.length < byte_length + width {
        return Err(failure(PathErrorCode::InvalidLength));
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, byte_length + width) };
    if bytes[byte_length..].iter().any(|byte| *byte != 0) {
        return Err(failure(PathErrorCode::MissingTerminator));
    }
    if platform == WINDOWS {
        let (units, _) = bytes[..byte_length].as_chunks::<2>();
        validate_windows_units(units.iter().map(|pair| u16::from_ne_bytes(*pair)))
            .map_err(failure)?;
    } else if bytes[..byte_length].contains(&0) {
        return Err(failure(PathErrorCode::EmbeddedNull));
    }
    Ok((&bytes[..byte_length], platform))
}

unsafe fn buffer_mut(handle: BufferHandle) -> Result<&'static mut ActusBuffer, i32> {
    if handle.is_null() {
        return Err(failure(PathErrorCode::InvalidLength));
    }
    let buffer = unsafe { &mut *handle };
    if buffer.length > buffer.capacity || (buffer.length > 0 && buffer.data.is_null()) {
        return Err(failure(PathErrorCode::InvalidLength));
    }
    Ok(buffer)
}

unsafe fn replace_payload(path: *mut ActusPath, bytes: &[u8]) -> Result<(), i32> {
    let path_ref = unsafe { &mut *path };
    let buffer = unsafe { buffer_mut(path_ref.storage) }?;
    let width = if path_ref.platform == POSIX { 1 } else { 2 };
    let required = bytes.len() + width;
    if required > buffer.capacity {
        let mut storage =
            unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
        storage
            .try_reserve_exact(required - storage.len())
            .map_err(|_| failure(PathErrorCode::CapacityExceeded))?;
        restore_buffer(buffer, storage);
    }
    let buffer = unsafe { buffer_mut(path_ref.storage) }?;
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.data, bytes.len()) };
    for index in bytes.len()..required {
        unsafe { *buffer.data.add(index) = 0 };
    }
    buffer.length = required;
    path_ref.length =
        i32::try_from(bytes.len() / width).map_err(|_| failure(PathErrorCode::CapacityExceeded))?;
    path_ref.capacity =
        i32::try_from(buffer.capacity).map_err(|_| failure(PathErrorCode::CapacityExceeded))?;
    path_ref.terminated = 1;
    Ok(())
}

fn compatible(left: i32, right: i32) -> Result<(), i32> {
    (left == right).then_some(()).ok_or(failure(PathErrorCode::UnsupportedPlatform))
}

fn separator(bytes: &[u8], platform: i32, offset: usize, width: usize) -> bool {
    let unit = if width == 1 {
        bytes[offset] as u16
    } else {
        u16::from_ne_bytes([bytes[offset], bytes[offset + 1]])
    };
    unit == b'/' as u16 || (platform == WINDOWS && unit == b'\\' as u16)
}

fn absolute(bytes: &[u8], platform: i32, width: usize) -> bool {
    if bytes.len() < width {
        return false;
    }
    if separator(bytes, platform, 0, width) {
        return true;
    }
    platform == WINDOWS
        && bytes.len() >= width * 3
        && read_unit(bytes, width, width) == b':' as u16
        && separator(bytes, platform, width * 2, width)
}

fn append_separator(output: &mut Vec<u8>, platform: i32, width: usize) {
    if width == 1 {
        output.push(b'/');
    } else {
        let unit = if platform == WINDOWS { b'\\' as u16 } else { b'/' as u16 };
        output.extend_from_slice(&unit.to_ne_bytes());
    }
}

fn trim_leading_separators(bytes: &[u8], platform: i32, width: usize) -> &[u8] {
    let mut offset = 0;
    while offset < bytes.len() && separator(bytes, platform, offset, width) {
        offset += width;
    }
    &bytes[offset..]
}

unsafe fn join_payload(path: *mut ActusPath, other: *const ActusPath) -> Result<(), i32> {
    let (left, platform) = unsafe { payload(path.cast_const()) }?;
    let (right, other_platform) = unsafe { payload(other) }?;
    compatible(platform, other_platform)?;
    let width = if platform == POSIX { 1 } else { 2 };
    let mut output = Vec::with_capacity(left.len() + width + right.len());
    if absolute(right, platform, width) {
        output.extend_from_slice(right);
    } else {
        output.extend_from_slice(left);
        if !output.is_empty() && !separator(&output, platform, output.len() - width, width) {
            append_separator(&mut output, platform, width);
        }
        output.extend_from_slice(trim_leading_separators(right, platform, width));
    }
    unsafe { replace_payload(path, &output) }
}

unsafe fn replace_file_name(path: *mut ActusPath, name: *const ActusPath) -> Result<(), i32> {
    let (current, platform) = unsafe { payload(path.cast_const()) }?;
    let (replacement, replacement_platform) = unsafe { payload(name) }?;
    compatible(platform, replacement_platform)?;
    let width = if platform == POSIX { 1 } else { 2 };
    if replacement.chunks_exact(width).any(|unit| separator(unit, platform, 0, width)) {
        return Err(failure(PathErrorCode::InvalidLength));
    }
    let end = current.len();
    let mut start = end;
    while start >= width && !separator(current, platform, start - width, width) {
        start -= width;
    }
    let mut output = Vec::with_capacity(start + replacement.len());
    output.extend_from_slice(&current[..start]);
    output.extend_from_slice(replacement);
    unsafe { replace_payload(path, &output) }
}

unsafe fn replace_extension(path: *mut ActusPath, extension: *const ActusPath) -> Result<(), i32> {
    let (current, platform) = unsafe { payload(path.cast_const()) }?;
    let (suffix, suffix_platform) = unsafe { payload(extension) }?;
    compatible(platform, suffix_platform)?;
    let width = if platform == POSIX { 1 } else { 2 };
    if suffix.chunks_exact(width).any(|unit| separator(unit, platform, 0, width)) {
        return Err(failure(PathErrorCode::InvalidLength));
    }
    let mut end = current.len();
    let mut start = end;
    while start >= width && !separator(current, platform, start - width, width) {
        start -= width;
    }
    let mut dot = end;
    let mut cursor = end;
    while cursor >= start + width {
        cursor -= width;
        if read_unit(current, cursor, width) == b'.' as u16 {
            dot = cursor;
            break;
        }
    }
    if dot == start || (dot == start + width && read_unit(current, start, width) == b'.' as u16) {
        dot = end;
    }
    end = dot;
    let mut output =
        Vec::with_capacity(end + if suffix.is_empty() { 0 } else { width + suffix.len() });
    output.extend_from_slice(&current[..end]);
    if !suffix.is_empty() {
        append_dot(&mut output, width);
        output.extend_from_slice(suffix);
    }
    unsafe { replace_payload(path, &output) }
}

fn read_unit(bytes: &[u8], offset: usize, width: usize) -> u16 {
    if width == 1 {
        bytes[offset] as u16
    } else {
        u16::from_ne_bytes([bytes[offset], bytes[offset + 1]])
    }
}

fn append_dot(output: &mut Vec<u8>, width: usize) {
    if width == 1 {
        output.push(b'.');
    } else {
        output.extend_from_slice(&(b'.' as u16).to_ne_bytes());
    }
}

#[unsafe(no_mangle)]
/// Reserves raw storage bytes for a mutable path without changing its payload.
///
/// # Safety
/// `path` must be null or a valid mutable `ActusPath` with owned storage.
pub unsafe extern "C" fn actus_path_reserve(path: *mut ActusPath, capacity: i32) -> i32 {
    if path.is_null() || capacity < 0 {
        return failure(PathErrorCode::InvalidLength);
    }
    let path_ref = unsafe { &mut *path };
    let Ok(buffer) = (unsafe { buffer_mut(path_ref.storage) }) else {
        return failure(PathErrorCode::InvalidLength);
    };
    let requested = capacity as usize;
    if requested <= buffer.capacity {
        path_ref.capacity = match i32::try_from(buffer.capacity) {
            Ok(capacity) => capacity,
            Err(_) => return failure(PathErrorCode::CapacityExceeded),
        };
        return 0;
    }
    let mut storage = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
    if storage.try_reserve_exact(requested - storage.len()).is_err() {
        restore_buffer(buffer, storage);
        return failure(PathErrorCode::CapacityExceeded);
    }
    restore_buffer(buffer, storage);
    path_ref.capacity = match i32::try_from(buffer.capacity) {
        Ok(capacity) => capacity,
        Err(_) => return failure(PathErrorCode::CapacityExceeded),
    };
    0
}

#[unsafe(no_mangle)]
/// Joins a borrowed path onto mutable owned path storage.
///
/// # Safety
/// Both pointers must be null or valid path descriptors; `path` must be mutable.
pub unsafe extern "C" fn actus_path_join(path: *mut ActusPath, other: *const ActusPath) -> i32 {
    unsafe { join_payload(path, other) }.map_or_else(|error| error, |_| 0)
}

#[unsafe(no_mangle)]
/// Pushes a borrowed path onto mutable owned path storage.
///
/// # Safety
/// Both pointers must be null or valid path descriptors; `path` must be mutable.
pub unsafe extern "C" fn actus_path_push(path: *mut ActusPath, other: *const ActusPath) -> i32 {
    unsafe { join_payload(path, other) }.map_or_else(|error| error, |_| 0)
}

#[unsafe(no_mangle)]
/// Replaces the final file-name component in mutable path storage.
///
/// # Safety
/// Both pointers must be null or valid path descriptors; `path` must be mutable.
pub unsafe extern "C" fn actus_path_set_file_name(
    path: *mut ActusPath,
    name: *const ActusPath,
) -> i32 {
    unsafe { replace_file_name(path, name) }.map_or_else(|error| error, |_| 0)
}

#[unsafe(no_mangle)]
/// Replaces the final extension in mutable path storage.
///
/// # Safety
/// Both pointers must be null or valid path descriptors; `path` must be mutable.
pub unsafe extern "C" fn actus_path_set_extension(
    path: *mut ActusPath,
    extension: *const ActusPath,
) -> i32 {
    unsafe { replace_extension(path, extension) }.map_or_else(|error| error, |_| 0)
}

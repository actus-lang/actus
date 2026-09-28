use super::super::types::BufferHandle;

use super::validation::{path_length, path_units, valid_buffer, validate_windows_units};
use super::{
    ActusPath, PosixRoot, is_ascii_drive, is_windows_separator, posix_is_separator, posix_root,
    validate_posix, windows_is_separator,
};

#[unsafe(no_mangle)]
/// Validates raw POSIX bytes or native-endian UTF-16 units in an Actus buffer.
///
/// # Safety
/// `storage` must be null or a live `ActusBuffer` with readable storage.
pub unsafe extern "C" fn actus_path_validate_storage(
    storage: BufferHandle,
    unit_width: i32,
) -> i32 {
    let Some(buffer) = (unsafe { valid_buffer(storage) }) else { return -4 };
    if unit_width != 1 && unit_width != 2 {
        return -2;
    }
    if buffer.capacity == 0 || buffer.length == 0 || buffer.length > buffer.capacity {
        return -4;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    if unit_width == 1 {
        if bytes.last() != Some(&0) {
            return -3;
        }
        let payload = &bytes[..bytes.len() - 1];
        return validate_posix(payload).map_or(-1, |_| 0);
    }
    if bytes.len() < 2 || bytes.len() % 2 != 0 || bytes[bytes.len() - 2..] != [0, 0] {
        return -3;
    }
    let payload = &bytes[..bytes.len() - 2];
    if payload.len() % 2 != 0 {
        return -2;
    }
    let (pairs, _) = payload.as_chunks::<2>();
    validate_windows_units(pairs.iter().map(|pair| u16::from_ne_bytes(*pair)))
        .map_or_else(|error| error as i32, |_| 0)
}

#[unsafe(no_mangle)]
/// Returns the payload length of a terminated raw path buffer.
///
/// # Safety
/// `storage` must be null or a live `ActusBuffer` with readable storage.
pub unsafe extern "C" fn actus_path_payload_length(storage: BufferHandle) -> i32 {
    let Some(buffer) = (unsafe { valid_buffer(storage) }) else { return -1 };
    if buffer.length == 0 || unsafe { *buffer.data.add(buffer.length - 1) } != 0 {
        return -1;
    }
    if buffer.length >= 2
        && buffer.length % 2 == 0
        && unsafe { *buffer.data.add(buffer.length - 2) } == 0
    {
        return i32::try_from((buffer.length - 2) / 2).unwrap_or(-1);
    }
    i32::try_from(buffer.length - 1).unwrap_or(-1)
}

#[unsafe(no_mangle)]
/// Returns the capacity of a raw path buffer.
///
/// # Safety
/// `storage` must be null or a live `ActusBuffer`.
pub unsafe extern "C" fn actus_path_storage_capacity(storage: BufferHandle) -> i32 {
    let Some(buffer) = (unsafe { valid_buffer(storage) }) else { return -1 };
    i32::try_from(buffer.capacity).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_posix_is_separator(unit: i32) -> i32 {
    i32::from(posix_is_separator(unit as u16))
}

#[unsafe(no_mangle)]
pub extern "C" fn actus_windows_is_separator(unit: i32) -> i32 {
    i32::from(windows_is_separator(unit as u16))
}

#[unsafe(no_mangle)]
/// Classifies a POSIX root from a borrowed C-layout path.
///
/// # Safety
/// `path` must be null or a valid pointer to an `ActusPath` whose storage is live.
pub unsafe extern "C" fn actus_posix_root_kind(path: *const ActusPath) -> i32 {
    let Ok((buffer, platform)) = (unsafe { path_units(path) }) else { return -2 };
    if platform != 0 {
        return -2;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, path_length(path, 1)) };
    match posix_root(bytes) {
        Ok(PosixRoot::Relative) => 0,
        Ok(PosixRoot::Absolute) => 1,
        Err(error) => error as i32,
    }
}

#[unsafe(no_mangle)]
/// Classifies a Windows root from a borrowed C-layout path.
///
/// # Safety
/// `path` must be null or a valid pointer to an `ActusPath` whose storage is live.
pub unsafe extern "C" fn actus_windows_root_kind(path: *const ActusPath) -> i32 {
    let Ok((buffer, platform)) = (unsafe { path_units(path) }) else { return -3 };
    if platform != 1 {
        return -2;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, path_length(path, 2)) };
    let (pairs, _) = bytes.as_chunks::<2>();
    let units = pairs.iter().map(|pair| u16::from_ne_bytes(*pair));
    if let Err(error) = validate_windows_units(units.clone()) {
        return error as i32;
    }
    let mut units = units.peekable();
    let first = units.next();
    let second = units.next();
    if first
        .zip(second)
        .is_some_and(|(left, right)| is_windows_separator(left) && is_windows_separator(right))
    {
        return 3;
    }
    if first.is_some_and(is_ascii_drive) && second == Some(b':' as u16) {
        return if units.peek().is_some_and(|unit| is_windows_separator(*unit)) { 2 } else { 1 };
    }
    0
}

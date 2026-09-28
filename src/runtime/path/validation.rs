use super::super::types::{ActusBuffer, BufferHandle};

use super::{ActusPath, PathErrorCode};

pub fn validate_posix(bytes: &[u8]) -> Result<(), PathErrorCode> {
    if bytes.contains(&0) { Err(PathErrorCode::EmbeddedNull) } else { Ok(()) }
}

pub fn validate_windows(units: &[u16]) -> Result<(), PathErrorCode> {
    validate_windows_units(units.iter().copied())
}

pub(crate) fn validate_windows_units<I>(units: I) -> Result<(), PathErrorCode>
where
    I: Iterator<Item = u16>,
{
    let mut pending_high = None;
    for unit in units {
        if unit == 0 {
            return Err(PathErrorCode::EmbeddedNull);
        }
        if let Some(high) = pending_high.take() {
            if !(0xDC00..=0xDFFF).contains(&unit) {
                return Err(PathErrorCode::InvalidEncoding);
            }
            let _ = high;
        } else if (0xD800..=0xDBFF).contains(&unit) {
            pending_high = Some(unit);
        } else if (0xDC00..=0xDFFF).contains(&unit) {
            return Err(PathErrorCode::InvalidEncoding);
        }
    }
    if pending_high.is_some() {
        return Err(PathErrorCode::InvalidEncoding);
    }
    Ok(())
}

pub(crate) unsafe fn valid_buffer(handle: BufferHandle) -> Option<&'static ActusBuffer> {
    if handle.is_null() || !(handle as usize).is_multiple_of(std::mem::align_of::<ActusBuffer>()) {
        return None;
    }
    let buffer = unsafe { &*handle };
    if buffer.length > buffer.capacity || (buffer.length > 0 && buffer.data.is_null()) {
        return None;
    }
    Some(buffer)
}

pub(crate) unsafe fn path_units(
    path: *const ActusPath,
) -> Result<(&'static ActusBuffer, i32), i32> {
    if path.is_null() {
        return Err(PathErrorCode::InvalidLength as i32);
    }
    let path = unsafe { &*path };
    let buffer =
        unsafe { valid_buffer(path.storage) }.ok_or(PathErrorCode::InvalidLength as i32)?;
    if path.length < 0 || path.capacity < path.length || path.terminated != 1 {
        return Err(PathErrorCode::InvalidLength as i32);
    }
    if path.platform != 0 && path.platform != 1 {
        return Err(PathErrorCode::InvalidEncoding as i32);
    }
    let required =
        if path.platform == 0 { path.length as usize + 1 } else { path.length as usize * 2 + 2 };
    if required > buffer.length || required > buffer.capacity {
        return Err(PathErrorCode::InvalidLength as i32);
    }
    Ok((buffer, path.platform))
}

pub(crate) unsafe fn path_length(path: *const ActusPath, unit_width: usize) -> usize {
    unsafe { (*path).length as usize * unit_width }
}

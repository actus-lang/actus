use super::path::{ActusPath, PathErrorCode, path_units, validate_windows_units};

#[repr(C)]
/// Borrowed POSIX path bytes for a native C ABI call.
///
/// `data` points into the source `ActusPath` storage and includes the trailing
/// null byte at `data[length]`. The descriptor owns no memory and is valid only
/// while the source path remains alive and unmodified.
pub struct ActusPathCView {
    pub data: *const u8,
    pub length: i32,
}

#[repr(C)]
/// Borrowed Windows UTF-16 units for a native wide C ABI call.
///
/// `data` points into the source `ActusPath` storage and includes the trailing
/// null unit at `data[length]`. Units use the target native byte order; no
/// UTF-8 conversion or allocation is performed.
pub struct ActusPathWideCView {
    pub data: *const u16,
    pub length: i32,
}

unsafe fn validated_payload(
    path: *const ActusPath,
    expected_platform: i32,
) -> Result<(*const u8, i32), i32> {
    let (buffer, platform) = unsafe { path_units(path) }?;
    if platform != expected_platform {
        return Err(PathErrorCode::UnsupportedPlatform as i32);
    }
    let path_ref = unsafe { &*path };
    let width = if platform == 0 { 1 } else { 2 };
    let payload_bytes = usize::try_from(path_ref.length)
        .map_err(|_| PathErrorCode::InvalidLength as i32)?
        .checked_mul(width)
        .ok_or(PathErrorCode::InvalidLength as i32)?;
    let required = payload_bytes + width;
    if buffer.length < required || buffer.capacity < required {
        return Err(PathErrorCode::InvalidLength as i32);
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, required) };
    if bytes[payload_bytes..].iter().any(|byte| *byte != 0) {
        return Err(PathErrorCode::MissingTerminator as i32);
    }
    if platform == 0 {
        if bytes[..payload_bytes].contains(&0) {
            return Err(PathErrorCode::EmbeddedNull as i32);
        }
    } else {
        let (units, _) = bytes[..payload_bytes].as_chunks::<2>();
        validate_windows_units(units.iter().map(|pair| u16::from_ne_bytes(*pair)))
            .map_err(|error| error as i32)?;
        if buffer.data.align_offset(std::mem::align_of::<u16>()) != 0 {
            return Err(PathErrorCode::InvalidLength as i32);
        }
    }
    Ok((buffer.data, path_ref.length))
}

#[unsafe(no_mangle)]
/// Writes a zero-allocation borrowed POSIX byte view into `output`.
///
/// # Safety
/// `output` must be writable for one `ActusPathCView`, and `path` must be a
/// valid immutable path descriptor. The returned pointer is borrowed from
/// `path` and must not outlive or mutate the source path.
pub unsafe extern "C" fn actus_path_c_view(
    output: *mut ActusPathCView,
    path: *const ActusPath,
) -> *mut ActusPathCView {
    if output.is_null() {
        return std::ptr::null_mut();
    }
    let Ok((data, length)) = (unsafe { validated_payload(path, 0) }) else {
        return std::ptr::null_mut();
    };
    unsafe { output.write(ActusPathCView { data, length }) };
    output
}

#[unsafe(no_mangle)]
/// Writes a zero-allocation borrowed Windows UTF-16 view into `output`.
///
/// # Safety
/// `output` must be writable for one `ActusPathWideCView`, and `path` must be
/// a valid immutable Windows path descriptor. The returned pointer is borrowed
/// from `path` and must not outlive or mutate the source path.
pub unsafe extern "C" fn actus_path_wide_c_view(
    output: *mut ActusPathWideCView,
    path: *const ActusPath,
) -> *mut ActusPathWideCView {
    if output.is_null() {
        return std::ptr::null_mut();
    }
    let Ok((data, length)) = (unsafe { validated_payload(path, 1) }) else {
        return std::ptr::null_mut();
    };
    unsafe {
        output.write(ActusPathWideCView { data: data.cast(), length });
    }
    output
}

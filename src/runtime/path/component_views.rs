use super::{ActusPath, ComponentRange, path_units};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ActusPathComponent {
    pub source: *const ActusPath,
    pub offset: i32,
    pub length: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ActusPathComponents {
    pub source: *const ActusPath,
    pub offset: i32,
    pub limit: i32,
}

struct RawPath<'a> {
    path: *const ActusPath,
    bytes: &'a [u8],
    width: usize,
    windows: bool,
    units: usize,
}

#[unsafe(no_mangle)]
/// Writes the final parent component into the caller-provided result slot.
///
/// The hidden result slot is first in the C ABI, followed by the borrowed
/// `Path` pointer. A non-null return is the same slot; null means no parent or
/// invalid path metadata. No allocation or text conversion occurs.
///
/// # Safety
/// `path` must be null or point to a live validated path, and `output` must be
/// null or point to writable caller-owned descriptor storage.
pub unsafe extern "C" fn actus_path_parent(
    output: *mut ActusPathComponent,
    path: *const ActusPath,
) -> *mut ActusPathComponent {
    let Some(raw) = (unsafe { raw_path(path) }) else { return std::ptr::null_mut() };
    let Some(range) = parent_range(&raw) else { return std::ptr::null_mut() };
    unsafe { write_component(output, raw.path, range, raw.width) }
}

#[unsafe(no_mangle)]
/// Writes the final file-name component into the caller-provided result slot.
///
/// # Safety
/// `path` must be null or point to a live validated path, and `output` must be
/// null or point to writable caller-owned descriptor storage.
pub unsafe extern "C" fn actus_path_file_name(
    output: *mut ActusPathComponent,
    path: *const ActusPath,
) -> *mut ActusPathComponent {
    unsafe { write_named_range(output, path, file_name_range) }
}

#[unsafe(no_mangle)]
/// Writes the final file stem, excluding its final non-leading extension.
///
/// # Safety
/// `path` must be null or point to a live validated path, and `output` must be
/// null or point to writable caller-owned descriptor storage.
pub unsafe extern "C" fn actus_path_file_stem(
    output: *mut ActusPathComponent,
    path: *const ActusPath,
) -> *mut ActusPathComponent {
    unsafe { write_named_range(output, path, stem_range) }
}

#[unsafe(no_mangle)]
/// Writes the final file extension without allocating or transcoding.
///
/// # Safety
/// `path` must be null or point to a live validated path, and `output` must be
/// null or point to writable caller-owned descriptor storage.
pub unsafe extern "C" fn actus_path_extension(
    output: *mut ActusPathComponent,
    path: *const ActusPath,
) -> *mut ActusPathComponent {
    unsafe { write_named_range(output, path, extension_range) }
}

#[unsafe(no_mangle)]
/// Initializes a borrowed component iterator in the caller-provided slot.
///
/// # Safety
/// `path` must be null or point to a live validated path, and `output` must be
/// null or point to writable caller-owned iterator storage.
pub unsafe extern "C" fn actus_path_components(
    output: *mut ActusPathComponents,
    path: *const ActusPath,
) {
    let Some(raw) = (unsafe { raw_path(path) }) else { return };
    if output.is_null() {
        return;
    }
    unsafe {
        *output = ActusPathComponents { source: raw.path, offset: 0, limit: raw.units as i32 };
    }
}

#[unsafe(no_mangle)]
/// Advances a borrowed iterator and writes its next component into a slot.
///
/// # Safety
/// `iterator` must be null or point to writable iterator storage whose source
/// path remains live; `output` must be null or point to writable descriptor
/// storage.
pub unsafe extern "C" fn actus_path_next_component(
    output: *mut ActusPathComponent,
    iterator: *mut ActusPathComponents,
) -> *mut ActusPathComponent {
    if iterator.is_null() {
        return std::ptr::null_mut();
    }
    let state = unsafe { &mut *iterator };
    let Some(raw) = (unsafe { raw_path(state.source) }) else { return std::ptr::null_mut() };
    if state.limit < 0 || state.offset < 0 || state.offset > state.limit {
        return std::ptr::null_mut();
    }
    let cursor = state.offset as usize * raw.width;
    let limit = (state.limit as usize).min(raw.units) * raw.width;
    let Some(range) = next_range(&raw, cursor, limit) else {
        state.offset = limit as i32;
        return std::ptr::null_mut();
    };
    state.offset = (range.end / raw.width) as i32;
    unsafe { write_component(output, raw.path, range, raw.width) }
}

unsafe fn write_named_range(
    output: *mut ActusPathComponent,
    path: *const ActusPath,
    select: impl FnOnce(&RawPath<'_>) -> Option<ComponentRange>,
) -> *mut ActusPathComponent {
    let Some(raw) = (unsafe { raw_path(path) }) else { return std::ptr::null_mut() };
    let Some(range) = select(&raw) else { return std::ptr::null_mut() };
    unsafe { write_component(output, raw.path, range, raw.width) }
}

unsafe fn write_component(
    output: *mut ActusPathComponent,
    source: *const ActusPath,
    range: ComponentRange,
    width: usize,
) -> *mut ActusPathComponent {
    if output.is_null() || range.end < range.start {
        return std::ptr::null_mut();
    }
    let offset = i32::try_from(range.start / width).ok();
    let length = i32::try_from((range.end - range.start) / width).ok();
    let (Some(offset), Some(length)) = (offset, length) else {
        return std::ptr::null_mut();
    };
    unsafe {
        *output = ActusPathComponent { source, offset, length };
    }
    output
}

unsafe fn raw_path(path: *const ActusPath) -> Option<RawPath<'static>> {
    let (buffer, platform) = unsafe { path_units(path) }.ok()?;
    let width = if platform == 0 { 1 } else { 2 };
    let length = unsafe { (*path).length as usize };
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, length * width) };
    let terminator = length * width;
    if platform == 0 {
        if unsafe { *buffer.data.add(terminator) } != 0 {
            return None;
        }
    } else if unsafe { *buffer.data.add(terminator) } != 0
        || unsafe { *buffer.data.add(terminator + 1) } != 0
    {
        return None;
    }
    if platform == 0 {
        super::validate_posix(bytes).ok()?;
    } else {
        let (pairs, _) = bytes.as_chunks::<2>();
        super::validate_windows_units(pairs.iter().map(|pair| u16::from_ne_bytes(*pair))).ok()?;
    }
    Some(RawPath { path, bytes, width, windows: platform == 1, units: length })
}

fn parent_range(raw: &RawPath<'_>) -> Option<ComponentRange> {
    let mut previous = None;
    let mut last = None;
    let mut cursor = 0;
    while let Some(range) = next_range(raw, cursor, raw.units * raw.width) {
        previous = last;
        last = Some(range);
        cursor = range.end;
    }
    previous
}

fn file_name_range(raw: &RawPath<'_>) -> Option<ComponentRange> {
    let mut cursor = 0;
    let mut last = None;
    while let Some(range) = next_range(raw, cursor, raw.units * raw.width) {
        last = Some(range);
        cursor = range.end;
    }
    last
}

fn stem_range(raw: &RawPath<'_>) -> Option<ComponentRange> {
    let file = file_name_range(raw)?;
    let dot = last_dot(raw, file)?;
    (dot > file.start).then_some(ComponentRange { start: file.start, end: dot })
}

fn extension_range(raw: &RawPath<'_>) -> Option<ComponentRange> {
    let file = file_name_range(raw)?;
    let dot = last_dot(raw, file)?;
    (dot > file.start && dot + raw.width < file.end)
        .then_some(ComponentRange { start: dot + raw.width, end: file.end })
}

fn last_dot(raw: &RawPath<'_>, file: ComponentRange) -> Option<usize> {
    let mut cursor = file.end;
    while cursor > file.start {
        cursor -= raw.width;
        if unit_at(raw, cursor) == b'.' as u16 {
            return Some(cursor);
        }
    }
    None
}

fn next_range(raw: &RawPath<'_>, mut cursor: usize, limit: usize) -> Option<ComponentRange> {
    while cursor < limit && is_separator(raw, cursor) {
        cursor += raw.width;
    }
    let start = cursor;
    while cursor < limit && !is_separator(raw, cursor) {
        cursor += raw.width;
    }
    (start < cursor).then_some(ComponentRange { start, end: cursor })
}

fn is_separator(raw: &RawPath<'_>, offset: usize) -> bool {
    let unit = unit_at(raw, offset);
    unit == b'/' as u16 || (raw.windows && unit == b'\\' as u16)
}

fn unit_at(raw: &RawPath<'_>, offset: usize) -> u16 {
    if raw.width == 1 {
        raw.bytes[offset] as u16
    } else {
        u16::from_ne_bytes([raw.bytes[offset], raw.bytes[offset + 1]])
    }
}

use super::types::{ActusBuffer, BufferHandle};

const POSIX_SEPARATOR: u8 = b'/';
const WINDOWS_SEPARATOR: u16 = b'/' as u16;
const WINDOWS_BACKSLASH: u16 = b'\\' as u16;

#[repr(C)]
pub struct ActusPath {
    pub storage: BufferHandle,
    pub platform: i32,
    pub length: i32,
    pub capacity: i32,
    pub terminated: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathErrorCode {
    EmbeddedNull = -1,
    InvalidEncoding = -2,
    MissingTerminator = -3,
    InvalidLength = -4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PosixRoot {
    Relative,
    Absolute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsRoot {
    Relative,
    DriveRelative,
    DriveAbsolute,
    Unc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentRange {
    pub start: usize,
    pub end: usize,
}

pub fn posix_root(bytes: &[u8]) -> Result<PosixRoot, PathErrorCode> {
    validate_posix(bytes)?;
    Ok(if bytes.first() == Some(&POSIX_SEPARATOR) {
        PosixRoot::Absolute
    } else {
        PosixRoot::Relative
    })
}

pub fn posix_is_separator(unit: u16) -> bool {
    unit == u16::from(POSIX_SEPARATOR)
}

pub fn windows_is_separator(unit: u16) -> bool {
    is_windows_separator(unit)
}

pub fn windows_root(units: &[u16]) -> Result<WindowsRoot, PathErrorCode> {
    validate_windows(units)?;
    if units.len() >= 2 && is_windows_separator(units[0]) && is_windows_separator(units[1]) {
        return Ok(WindowsRoot::Unc);
    }
    if units.len() >= 2 && is_ascii_drive(units[0]) && units[1] == b':' as u16 {
        return Ok(if units.get(2).is_some_and(|unit| is_windows_separator(*unit)) {
            WindowsRoot::DriveAbsolute
        } else {
            WindowsRoot::DriveRelative
        });
    }
    Ok(WindowsRoot::Relative)
}

pub fn validate_posix(bytes: &[u8]) -> Result<(), PathErrorCode> {
    if bytes.contains(&0) { Err(PathErrorCode::EmbeddedNull) } else { Ok(()) }
}

pub fn validate_windows(units: &[u16]) -> Result<(), PathErrorCode> {
    validate_windows_units(units.iter().copied())
}

fn validate_windows_units<I>(units: I) -> Result<(), PathErrorCode>
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

pub fn components<'a>(units: &'a [u16], windows: bool) -> Components<'a> {
    Components { units, windows, cursor: 0 }
}

pub fn file_name(units: &[u16], windows: bool) -> Option<ComponentRange> {
    components(units, windows).last()
}

pub fn file_stem(units: &[u16], windows: bool) -> Option<ComponentRange> {
    let file = file_name(units, windows)?;
    let dot =
        units[file.start..file.end].iter().rposition(|unit| *unit == b'.' as u16)? + file.start;
    (dot > file.start).then_some(ComponentRange { start: file.start, end: dot })
}

pub fn extension(units: &[u16], windows: bool) -> Option<ComponentRange> {
    let file = file_name(units, windows)?;
    let dot =
        units[file.start..file.end].iter().rposition(|unit| *unit == b'.' as u16)? + file.start;
    (dot > file.start && dot + 1 < file.end)
        .then_some(ComponentRange { start: dot + 1, end: file.end })
}

pub struct Components<'a> {
    units: &'a [u16],
    windows: bool,
    cursor: usize,
}

impl<'a> Iterator for Components<'a> {
    type Item = ComponentRange;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.units.len() && self.is_separator(self.units[self.cursor]) {
            self.cursor += 1;
        }
        let start = self.cursor;
        while self.cursor < self.units.len() && !self.is_separator(self.units[self.cursor]) {
            self.cursor += 1;
        }
        (start < self.cursor).then_some(ComponentRange { start, end: self.cursor })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.units.len().saturating_sub(self.cursor)))
    }
}

impl Components<'_> {
    fn is_separator(&self, unit: u16) -> bool {
        unit == WINDOWS_SEPARATOR || (self.windows && unit == WINDOWS_BACKSLASH)
    }
}

fn is_windows_separator(unit: u16) -> bool {
    unit == WINDOWS_SEPARATOR || unit == WINDOWS_BACKSLASH
}

fn is_ascii_drive(unit: u16) -> bool {
    (b'a' as u16..=b'z' as u16).contains(&unit) || (b'A' as u16..=b'Z' as u16).contains(&unit)
}

unsafe fn valid_buffer(handle: BufferHandle) -> Option<&'static ActusBuffer> {
    if handle.is_null() {
        return None;
    }
    let buffer = unsafe { &*handle };
    if buffer.length > buffer.capacity || (buffer.length > 0 && buffer.data.is_null()) {
        return None;
    }
    Some(buffer)
}

unsafe fn path_units(path: *const ActusPath) -> Result<(&'static ActusBuffer, i32), i32> {
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
    if buffer.capacity == 0 || buffer.length == 0 || buffer.length >= buffer.capacity {
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
    validate_windows_units(pairs.iter().map(|pair| u16::from_ne_bytes(*pair))).map_or(-1, |_| 0)
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

unsafe fn path_length(path: *const ActusPath, unit_width: usize) -> usize {
    unsafe { (*path).length as usize * unit_width }
}

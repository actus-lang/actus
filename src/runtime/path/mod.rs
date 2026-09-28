use super::types::BufferHandle;

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
    CapacityExceeded = -5,
    UnsupportedPlatform = -6,
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

mod abi;
mod builders;
mod c_abi_views;
mod component_views;
mod components;
mod normalize;
mod predicates;
mod validation;

pub use abi::{
    actus_path_payload_length, actus_path_storage_capacity, actus_path_validate_storage,
    actus_posix_is_separator, actus_posix_root_kind, actus_windows_is_separator,
    actus_windows_root_kind,
};
pub use builders::{
    actus_path_join, actus_path_push, actus_path_reserve, actus_path_set_extension,
    actus_path_set_file_name,
};
pub use c_abi_views::{
    ActusPathCView, ActusPathWideCView, actus_path_c_view, actus_path_wide_c_view,
};
pub use component_views::{
    ActusPathComponent, ActusPathComponents, actus_path_components, actus_path_extension,
    actus_path_file_name, actus_path_file_stem, actus_path_next_component, actus_path_parent,
};
pub use components::{Components, components, extension, file_name, file_stem};
pub use normalize::actus_path_normalize;
pub use predicates::{
    actus_path_ends_with, actus_path_has_root, actus_path_is_absolute, actus_path_is_relative,
    actus_path_starts_with,
};
pub(crate) use validation::{path_length, path_units, validate_windows_units};
pub use validation::{validate_posix, validate_windows};

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

fn is_windows_separator(unit: u16) -> bool {
    unit == WINDOWS_SEPARATOR || unit == WINDOWS_BACKSLASH
}

fn is_ascii_drive(unit: u16) -> bool {
    (b'a' as u16..=b'z' as u16).contains(&unit) || (b'A' as u16..=b'Z' as u16).contains(&unit)
}

mod abi;
mod ascii;
mod builders;
mod c_abi_views;
mod component_views;
mod components;
mod model;
mod normalize;
mod predicates;
mod validation;

pub use abi::{
    actus_path_payload_length, actus_path_storage_capacity, actus_path_validate_storage,
    actus_posix_is_separator, actus_posix_root_kind, actus_windows_is_separator,
    actus_windows_root_kind,
};
pub use ascii::actus_path_ascii_to_native;
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
    actus_path_ends_with, actus_path_extension_equals, actus_path_has_root, actus_path_is_absolute,
    actus_path_is_hidden, actus_path_is_relative, actus_path_starts_with,
};
pub(crate) use validation::{path_length, path_units, validate_windows_units};
pub use validation::{validate_posix, validate_windows};

pub use model::{
    ActusPath, ComponentRange, PathErrorCode, PosixRoot, WindowsRoot, posix_is_separator,
    posix_root, windows_is_separator, windows_root,
};
use model::{WINDOWS_BACKSLASH, WINDOWS_SEPARATOR, is_ascii_drive, is_windows_separator};

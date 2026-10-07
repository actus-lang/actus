use std::path::Path;

mod allocation;
mod capabilities;
mod console;
mod contract;
mod fs;
mod input;
mod path;
mod region;
mod serialization;
mod stream;
mod time;
mod types;

pub use allocation::{
    actus_buffer_allocate, actus_buffer_append, actus_buffer_append_buffer,
    actus_buffer_append_range, actus_buffer_capacity, actus_buffer_clear, actus_buffer_drop,
    actus_buffer_reserve, actus_copy_buffer_reserve, actus_enum_allocate, actus_enum_drop,
};
pub use capabilities::{REGION_CAPABILITY_CAPACITY, RegionCapabilityTable, actus_region_drop};
pub use console::{
    actus_flush_stderr, actus_flush_stdout, actus_print_buffer_stderr, actus_print_buffer_stdout,
    actus_print_int, actus_print_int_stderr, actus_print_line_buffer_stderr,
    actus_print_line_buffer_stdout, actus_print_string, actus_print_string_stderr,
    actus_string_byte_at, actus_string_copy_to_buffer, actus_string_length,
    actus_write_string_stdout,
};
pub use contract::{
    ABI_HANDLE_FAILURE, ABI_STATUS_END_OF_STREAM, ABI_STATUS_FAILURE, ABI_STATUS_SUCCESS,
    BUFFER_ALLOCATE_SYMBOL, BUFFER_APPEND_SYMBOL, BUFFER_BYTE_AT_SYMBOL,
    BUFFER_CRC32_MATCHES_SYMBOL, BUFFER_CRC32_SYMBOL, BUFFER_DROP_SYMBOL, BUFFER_LENGTH_SYMBOL,
    BUFFER_RESERVE_SYMBOL, BUFFER_VALIDATE_FIXED_FRAME_SYMBOL, BUFFERED_WRITE_STDOUT_SYMBOL,
    ENUM_ALLOCATE_SYMBOL, ENUM_DROP_SYMBOL, FLUSH_BUFFERED_STDOUT_SYMBOL, FLUSH_STDERR_SYMBOL,
    FLUSH_STDOUT_SYMBOL, MONOTONIC_NANOS_SYMBOL, PRINT_BUFFER_STDERR_SYMBOL,
    PRINT_BUFFER_STDOUT_SYMBOL, PRINT_INT_STDERR_SYMBOL, PRINT_INT_SYMBOL,
    PRINT_LINE_BUFFER_STDERR_SYMBOL, PRINT_LINE_BUFFER_STDOUT_SYMBOL, PRINT_STRING_STDERR_SYMBOL,
    PRINT_STRING_SYMBOL, READ_BYTE_SYMBOL, READ_STDIN_LINE_SYMBOL, REGION_DROP_SYMBOL,
    RUNTIME_ABI_VERSION, RuntimeCapability, SLEEP_CONTEXT_CRITICAL_SECTION,
    SLEEP_CONTEXT_ENTER_SYMBOL, SLEEP_CONTEXT_EXIT_SYMBOL, SLEEP_CONTEXT_INTERRUPT,
    SLEEP_NANOS_SYMBOL, STRING_BYTE_AT_SYMBOL, STRING_COPY_STATUS_CAPACITY,
    STRING_COPY_STATUS_INVALID_UTF8, STRING_LENGTH_SYMBOL, STRING_STATUS_INVALID_UTF8,
    STRING_STATUS_NULL, STRING_STATUS_OUT_OF_BOUNDS, WRITE_BUFFER_STDOUT_SYMBOL,
    WRITE_STRING_STDOUT_SYMBOL, is_successful_count,
};
pub use fs::{
    actus_file_close, actus_file_close_buffer, actus_file_copy_buffer, actus_file_copy_path,
    actus_file_create_dir_buffer, actus_file_create_dir_path, actus_file_flush,
    actus_file_flush_buffer, actus_file_metadata_buffer, actus_file_metadata_handle_buffer,
    actus_file_metadata_path, actus_file_open, actus_file_open_buffer,
    actus_file_open_options_buffer, actus_file_open_options_path, actus_file_open_path,
    actus_file_read, actus_file_read_buffer, actus_file_remove_buffer,
    actus_file_remove_dir_buffer, actus_file_remove_dir_path, actus_file_remove_path,
    actus_file_rename_buffer, actus_file_rename_path, actus_file_seek, actus_file_seek_buffer,
    actus_file_write, actus_file_write_buffer,
};
pub use input::{actus_read_byte, actus_read_stdin_line};
pub use path::{
    ActusPath, ActusPathCView, ActusPathComponent, ActusPathComponents, ActusPathWideCView,
    ComponentRange, Components, PathErrorCode, PosixRoot, WindowsRoot, actus_path_ascii_to_native,
    actus_path_c_view, actus_path_components, actus_path_ends_with, actus_path_extension,
    actus_path_file_name, actus_path_file_stem, actus_path_has_root, actus_path_is_absolute,
    actus_path_is_relative, actus_path_join, actus_path_next_component, actus_path_normalize,
    actus_path_parent, actus_path_payload_length, actus_path_push, actus_path_reserve,
    actus_path_set_extension, actus_path_set_file_name, actus_path_starts_with,
    actus_path_storage_capacity, actus_path_validate_storage, actus_path_wide_c_view,
    actus_posix_is_separator, actus_posix_root_kind, actus_windows_is_separator,
    actus_windows_root_kind, components, extension, file_name, file_stem, posix_is_separator,
    posix_root, validate_posix, validate_windows, windows_is_separator, windows_root,
};
pub use region::{
    InMemoryRegion, MutableRegionView, RegionAddressProfile, RegionDescriptor, RegionError,
    RegionGeneration, RegionHandle, RegionView, UNINITIALIZED_REGION_GENERATION,
};
pub use serialization::{
    actus_buffer_crc32, actus_buffer_crc32_matches, actus_buffer_validate_fixed_frame, crc32,
    validate_fixed_frame,
};
pub use stream::{
    actus_buffer_byte_at, actus_buffer_length, actus_buffer_validate_utf8,
    actus_buffered_write_stdout, actus_cursor_flush, actus_cursor_read, actus_cursor_seek,
    actus_cursor_write, actus_flush_buffered_stdout, actus_write_buffer_stdout,
};
pub use time::{
    actus_monotonic_nanos, actus_sleep_context_enter, actus_sleep_context_exit, actus_sleep_nanos,
};
pub use types::{ActusBuffer, ActusMetadata, BufferHandle};

pub fn runtime_archive_path() -> Option<&'static Path> {
    option_env!("ACTUS_RUNTIME_ARCHIVE").map(Path::new)
}

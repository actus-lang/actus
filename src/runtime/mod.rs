use std::path::Path;

mod allocation;
mod console;
mod contract;
mod fs;
mod input;
mod stream;
mod types;

pub use allocation::{
    actus_buffer_allocate, actus_buffer_append, actus_buffer_drop, actus_buffer_reserve,
    actus_enum_allocate, actus_enum_drop,
};
pub use console::{
    actus_flush_stderr, actus_flush_stdout, actus_print_buffer_stderr, actus_print_buffer_stdout,
    actus_print_int, actus_print_int_stderr, actus_print_line_buffer_stderr,
    actus_print_line_buffer_stdout, actus_print_string, actus_print_string_stderr,
};
pub use contract::{
    BUFFER_ALLOCATE_SYMBOL, BUFFER_APPEND_SYMBOL, BUFFER_DROP_SYMBOL, BUFFER_RESERVE_SYMBOL,
    BUFFERED_WRITE_STDOUT_SYMBOL, ENUM_ALLOCATE_SYMBOL, ENUM_DROP_SYMBOL,
    FLUSH_BUFFERED_STDOUT_SYMBOL, FLUSH_STDERR_SYMBOL, FLUSH_STDOUT_SYMBOL,
    PRINT_BUFFER_STDERR_SYMBOL, PRINT_BUFFER_STDOUT_SYMBOL, PRINT_INT_STDERR_SYMBOL,
    PRINT_INT_SYMBOL, PRINT_LINE_BUFFER_STDERR_SYMBOL, PRINT_LINE_BUFFER_STDOUT_SYMBOL,
    PRINT_STRING_STDERR_SYMBOL, PRINT_STRING_SYMBOL, READ_BYTE_SYMBOL, READ_STDIN_LINE_SYMBOL,
    RUNTIME_ABI_VERSION, RuntimeCapability, WRITE_BUFFER_STDOUT_SYMBOL,
};
pub use fs::{
    actus_file_close, actus_file_close_buffer, actus_file_flush, actus_file_flush_buffer,
    actus_file_open, actus_file_open_buffer, actus_file_read, actus_file_read_buffer,
    actus_file_write, actus_file_write_buffer,
};
pub use input::{actus_read_byte, actus_read_stdin_line};
pub use stream::{
    actus_buffer_length, actus_buffered_write_stdout, actus_cursor_flush, actus_cursor_read,
    actus_cursor_seek, actus_cursor_write, actus_flush_buffered_stdout, actus_write_buffer_stdout,
};
pub use types::{ActusBuffer, BufferHandle};

pub fn runtime_archive_path() -> Option<&'static Path> {
    option_env!("ACTUS_RUNTIME_ARCHIVE").map(Path::new)
}

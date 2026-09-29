use std::ptr;

use actus::runtime::{
    ABI_STATUS_FAILURE, ActusMetadata, actus_buffered_write_stdout, actus_cursor_flush,
    actus_cursor_read, actus_cursor_seek, actus_cursor_write, actus_file_copy_buffer,
    actus_file_create_dir_buffer, actus_file_flush_buffer, actus_file_metadata_buffer,
    actus_file_open_buffer, actus_file_open_options_buffer, actus_file_read_buffer,
    actus_file_remove_buffer, actus_file_remove_dir_buffer, actus_file_rename_buffer,
    actus_file_write_buffer, actus_flush_buffered_stdout, actus_print_buffer_stderr,
    actus_print_buffer_stdout, actus_read_stdin_line, actus_write_buffer_stdout,
};

#[test]
fn null_runtime_handles_return_explicit_failure_statuses() {
    let null = ptr::null_mut();
    let mut metadata = ActusMetadata { size: 0, is_file: 0, is_dir: 0, readonly: 0 };

    unsafe {
        assert_eq!(actus_print_buffer_stdout(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_print_buffer_stderr(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_write_buffer_stdout(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_buffered_write_stdout(null, null), ABI_STATUS_FAILURE);
        assert_eq!(actus_flush_buffered_stdout(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_read_stdin_line(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_cursor_read(null, 0, null), ABI_STATUS_FAILURE);
        assert_eq!(actus_cursor_write(null, 0, null), ABI_STATUS_FAILURE);
        assert_eq!(actus_cursor_seek(null, 0), ABI_STATUS_FAILURE);
        assert_eq!(actus_cursor_flush(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_open_buffer(null, 0), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_open_options_buffer(null, 0, 0, 0, 0, 0, 0), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_read_buffer(-1, null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_write_buffer(-1, null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_metadata_buffer(null, &mut metadata), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_copy_buffer(null, null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_rename_buffer(null, null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_remove_buffer(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_create_dir_buffer(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_remove_dir_buffer(null), ABI_STATUS_FAILURE);
        assert_eq!(actus_file_flush_buffer(-1), ABI_STATUS_FAILURE);
    }
}

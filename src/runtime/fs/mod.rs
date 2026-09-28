mod buffers;
mod file;
mod operations;
mod path_open;

pub use buffers::{
    actus_file_close_buffer, actus_file_flush_buffer, actus_file_open_buffer,
    actus_file_open_options_buffer, actus_file_read_buffer, actus_file_write_buffer,
};
pub use file::{
    actus_file_close, actus_file_flush, actus_file_open, actus_file_read, actus_file_seek,
    actus_file_seek_buffer, actus_file_write,
};
pub use operations::{
    actus_file_copy_buffer, actus_file_copy_path, actus_file_create_dir_buffer,
    actus_file_create_dir_path, actus_file_metadata_buffer, actus_file_metadata_handle_buffer,
    actus_file_metadata_path, actus_file_remove_buffer, actus_file_remove_dir_buffer,
    actus_file_remove_dir_path, actus_file_remove_path, actus_file_rename_buffer,
    actus_file_rename_path,
};
pub use path_open::{actus_file_open_options_path, actus_file_open_path};

pub(super) use file::{actus_handle, handle_to_actus, with_file};
pub(super) const MODE_CREATE: u32 = 1;

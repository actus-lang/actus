mod buffers;
mod directory;
mod file;
mod operations;
mod path_bridge;
mod path_open;
mod temporary;
mod walk;

pub use buffers::{
    actus_file_close_buffer, actus_file_flush_buffer, actus_file_open_buffer,
    actus_file_open_options_buffer, actus_file_read_buffer, actus_file_sync_all_buffer,
    actus_file_sync_data_buffer, actus_file_write_buffer,
};
pub use directory::{
    actus_directory_close, actus_directory_next, actus_directory_next_entry, actus_directory_open,
    actus_directory_open_ordered,
};
pub use file::{
    actus_file_close, actus_file_flush, actus_file_lock_exclusive, actus_file_lock_shared,
    actus_file_open, actus_file_read, actus_file_seek, actus_file_seek_buffer, actus_file_sync_all,
    actus_file_sync_data, actus_file_try_lock, actus_file_unlock, actus_file_write,
};
pub use operations::{
    actus_file_canonicalize_path, actus_file_copy_buffer, actus_file_copy_path,
    actus_file_create_dir_buffer, actus_file_create_dir_path, actus_file_metadata_buffer,
    actus_file_metadata_handle_buffer, actus_file_metadata_path, actus_file_remove_buffer,
    actus_file_remove_dir_buffer, actus_file_remove_dir_path, actus_file_remove_path,
    actus_file_rename_buffer, actus_file_rename_path, actus_file_same_path,
    actus_file_symlink_metadata_path,
};
pub use path_open::{actus_file_open_options_path, actus_file_open_path};
pub use temporary::{actus_directory_create_temporary_path, actus_file_open_temporary_path};
#[allow(unused_imports)]
pub use walk::{ActusWalkEvent, actus_walker_close, actus_walker_next, actus_walker_open};

pub(super) use file::{actus_handle, handle_to_actus, with_file};
pub(super) const MODE_CREATE: u32 = 1;

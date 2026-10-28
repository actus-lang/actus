use std::mem::ManuallyDrop;

#[repr(C)]
pub struct ActusBuffer {
    pub data: *mut u8,
    pub length: usize,
    pub capacity: usize,
}

#[repr(C)]
pub struct ActusMetadata {
    pub size: i32,
    pub is_file: i32,
    pub is_dir: i32,
    pub readonly: i32,
    pub is_symlink: i32,
    pub modified_seconds: i64,
    pub modified_status: i32,
    pub accessed_seconds: i64,
    pub accessed_status: i32,
    pub created_seconds: i64,
    pub created_status: i32,
    pub identity_primary: u64,
    pub identity_secondary: u64,
    pub identity_kind: i32,
    pub identity_status: i32,
}

pub type BufferHandle = *mut ActusBuffer;

pub(super) fn restore_buffer(buffer: &mut ActusBuffer, data: Vec<u8>) {
    let data = ManuallyDrop::new(data);
    buffer.data = data.as_ptr() as *mut u8;
    buffer.length = data.len();
    buffer.capacity = data.capacity();
}

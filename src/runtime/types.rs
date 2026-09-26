use std::mem::ManuallyDrop;

#[repr(C)]
pub struct ActusBuffer {
    pub data: *mut u8,
    pub length: usize,
    pub capacity: usize,
}

pub type BufferHandle = *mut ActusBuffer;

pub(super) fn restore_buffer(buffer: &mut ActusBuffer, data: Vec<u8>) {
    let data = ManuallyDrop::new(data);
    buffer.data = data.as_ptr() as *mut u8;
    buffer.length = data.len();
    buffer.capacity = data.capacity();
}

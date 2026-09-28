use super::super::contract::ABI_STATUS_FAILURE;
use super::super::types::BufferHandle;
#[cfg(windows)]
use super::super::types::restore_buffer;

const ASCII_LIMIT: u8 = 0x7f;

/// Converts a terminated ASCII byte path into native host path storage.
///
/// POSIX callers retain their byte representation. Windows callers receive
/// native-endian UTF-16 code units. The conversion rejects embedded nulls and
/// non-ASCII bytes because widening an arbitrary byte is not a valid encoding
/// contract for a Windows filesystem path.
///
/// # Safety
/// `storage` must be null or a live buffer returned by the Actus allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_path_ascii_to_native(storage: BufferHandle) -> i32 {
    let Some(buffer) = (unsafe { storage.as_mut() }) else {
        return ABI_STATUS_FAILURE;
    };
    if buffer.length == 0 || buffer.length > buffer.capacity || buffer.data.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    let Some(payload) = bytes.strip_suffix(&[0]) else { return ABI_STATUS_FAILURE };
    if payload.iter().any(|byte| *byte == 0 || *byte > ASCII_LIMIT) {
        return ABI_STATUS_FAILURE;
    }
    #[cfg(windows)]
    {
        let mut units = Vec::with_capacity((payload.len() + 1) * std::mem::size_of::<u16>());
        for byte in payload {
            units.extend_from_slice(&u16::from(*byte).to_ne_bytes());
        }
        units.extend_from_slice(&0u16.to_ne_bytes());
        if units.as_ptr().align_offset(std::mem::align_of::<u16>()) != 0 {
            return ABI_STATUS_FAILURE;
        }
        let data = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
        drop(data);
        restore_buffer(buffer, units);
        1
    }
    #[cfg(not(windows))]
    0
}

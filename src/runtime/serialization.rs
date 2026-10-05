use super::types::BufferHandle;

/// Computes the IEEE CRC32 of a validated byte range without allocating.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut checksum = u32::MAX;
    for byte in bytes {
        checksum ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(checksum & 1);
            checksum = (checksum >> 1) ^ (0xEDB8_8320u32 & mask);
        }
    }
    !checksum
}

/// Computes CRC32 for a caller-owned buffer range; returns -1 for invalid input.
///
/// # Safety
/// `handle` must be null or point to a live `ActusBuffer` whose fields describe
/// valid storage for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_crc32(handle: BufferHandle, start: i64, end: i64) -> i64 {
    let Some(buffer) = (unsafe { handle.as_ref() }) else { return -1 };
    if start < 0 || end < start {
        return -1;
    }
    let (start, end) = (start as usize, end as usize);
    if end > buffer.length || buffer.length > buffer.capacity {
        return -1;
    }
    if buffer.length > 0 && buffer.data.is_null() {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    i64::from(crc32(&bytes[start..end]))
}

/// Compares a validated buffer range with an expected IEEE CRC32 value.
///
/// # Safety
/// `handle` must be null or point to a live `ActusBuffer` whose fields describe
/// valid storage for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_crc32_matches(
    handle: BufferHandle,
    start: i64,
    end: i64,
    expected: i64,
) -> i64 {
    let actual = unsafe { actus_buffer_crc32(handle, start, end) };
    i64::from(actual >= 0 && actual == expected)
}

#[cfg(test)]
mod tests {
    use super::crc32;

    #[test]
    fn matches_the_ieee_crc32_reference_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn distinguishes_changed_payload_bytes() {
        assert_ne!(crc32(b"frame"), crc32(b"Frame"));
    }
}

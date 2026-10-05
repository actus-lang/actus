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

/// Validates a fixed binary frame without allocating or touching the filesystem.
#[allow(clippy::too_many_arguments)]
pub fn validate_fixed_frame(
    bytes: &[u8],
    little_endian: bool,
    version_offset: usize,
    expected_version: u16,
    payload_offset: usize,
    payload_length: usize,
    checksum_start: usize,
    checksum_end: usize,
    checksum_offset: usize,
) -> bool {
    let Some(version_bytes) = bytes.get(version_offset..version_offset.saturating_add(2)) else {
        return false;
    };
    let version = if little_endian {
        u16::from_le_bytes([version_bytes[0], version_bytes[1]])
    } else {
        u16::from_be_bytes([version_bytes[0], version_bytes[1]])
    };
    if version != expected_version
        || bytes.get(payload_offset..payload_offset.saturating_add(payload_length)).is_none()
    {
        return false;
    }
    let Some(checksum_bytes) = bytes.get(checksum_start..checksum_end) else { return false };
    let Some(stored_bytes) = bytes.get(checksum_offset..checksum_offset.saturating_add(4)) else {
        return false;
    };
    let stored = if little_endian {
        u32::from_le_bytes([stored_bytes[0], stored_bytes[1], stored_bytes[2], stored_bytes[3]])
    } else {
        u32::from_be_bytes([stored_bytes[0], stored_bytes[1], stored_bytes[2], stored_bytes[3]])
    };
    crc32(checksum_bytes) == stored
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

/// Validates a fixed frame described by explicit byte offsets and widths.
///
/// # Safety
/// `handle` must be null or point to a live `ActusBuffer` whose fields describe
/// valid storage for the duration of this call.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_buffer_validate_fixed_frame(
    handle: BufferHandle,
    little_endian: i64,
    version_offset: i64,
    expected_version: i64,
    payload_offset: i64,
    payload_length: i64,
    checksum_start: i64,
    checksum_end: i64,
    checksum_offset: i64,
) -> i64 {
    let Some(buffer) = (unsafe { handle.as_ref() }) else { return 0 };
    if buffer.length > buffer.capacity || (buffer.length > 0 && buffer.data.is_null()) {
        return 0;
    }
    if [version_offset, payload_offset, checksum_start, checksum_end, checksum_offset]
        .iter()
        .any(|value| *value < 0)
        || expected_version < 0
        || expected_version > i64::from(u16::MAX)
        || payload_length < 0
    {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    i64::from(validate_fixed_frame(
        bytes,
        little_endian != 0,
        version_offset as usize,
        expected_version as u16,
        payload_offset as usize,
        payload_length as usize,
        checksum_start as usize,
        checksum_end as usize,
        checksum_offset as usize,
    ))
}

#[cfg(test)]
mod tests {
    use super::{crc32, validate_fixed_frame};

    #[test]
    fn matches_the_ieee_crc32_reference_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn distinguishes_changed_payload_bytes() {
        assert_ne!(crc32(b"frame"), crc32(b"Frame"));
    }

    #[test]
    fn validates_version_payload_and_little_endian_checksum() {
        let mut bytes = vec![1, 0, 7, 8, 9, 0, 0, 0, 0];
        let checksum = crc32(&bytes[..5]).to_le_bytes();
        bytes[5..9].copy_from_slice(&checksum);
        assert!(validate_fixed_frame(&bytes, true, 0, 1, 2, 3, 0, 5, 5));
        assert!(!validate_fixed_frame(&bytes, true, 0, 2, 2, 3, 0, 5, 5));
    }
}

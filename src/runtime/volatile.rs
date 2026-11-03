//! Native runtime boundary for typed volatile reads.

use std::ptr;

use super::actus_enum_allocate;

const RESULT_OK: u32 = 0;
const RESULT_ERR: u32 = 1;
const RESULT_SCALAR_SIZE: usize = 8;
const RESULT_U64_SIZE: usize = 16;
const PAYLOAD_OFFSET: usize = 4;
const U64_PAYLOAD_OFFSET: usize = 8;
const DESCRIPTOR_MAGIC: u32 = 0x4156_4F4C;

/// Native descriptor supplied by a trusted platform provider.
///
/// The pointer crossing the Actus boundary is opaque. The provider owns the
/// descriptor and the addressed object; Actus can only borrow it for reads.
#[repr(C)]
pub struct VolatileAddressDescriptor {
    pub magic: u32,
    pub address: *const u8,
    pub byte_length: usize,
    pub alignment: usize,
    pub target_size: usize,
    pub target_alignment: usize,
    pub writable: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VolatileReadError {
    InvalidDescriptor,
    NullAddress,
    OffsetOverflow,
    OutOfBounds,
    Misaligned,
    TypeMismatch,
    UnsupportedWidth,
    AllocationFailure,
}

fn error_code(error: VolatileReadError) -> u32 {
    match error {
        VolatileReadError::InvalidDescriptor => 0,
        VolatileReadError::NullAddress => 1,
        VolatileReadError::OffsetOverflow => 2,
        VolatileReadError::OutOfBounds => 3,
        VolatileReadError::Misaligned => 4,
        VolatileReadError::TypeMismatch => 5,
        VolatileReadError::UnsupportedWidth => 6,
        VolatileReadError::AllocationFailure => 7,
    }
}

fn write_u32(destination: *mut u8, offset: usize, value: u32) {
    unsafe { ptr::write_unaligned(destination.add(offset).cast::<u32>(), value) };
}

fn write_u64(destination: *mut u8, offset: usize, value: u64) {
    unsafe { ptr::write_unaligned(destination.add(offset).cast::<u64>(), value) };
}

fn allocate_result(size: usize, discriminant: u32) -> *mut u8 {
    let result = actus_enum_allocate(size);
    if !result.is_null() {
        write_u32(result, 0, discriminant);
    }
    result
}

fn error_result(size: usize, error: VolatileReadError) -> *mut u8 {
    let result = allocate_result(size, RESULT_ERR);
    if !result.is_null() {
        write_u32(result, PAYLOAD_OFFSET, error_code(error));
    }
    result
}

fn validate(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    width: usize,
    target_size: u64,
    target_alignment: u64,
    require_write: bool,
) -> Result<*const u8, VolatileReadError> {
    let descriptor = unsafe { descriptor.as_ref() }.ok_or(VolatileReadError::InvalidDescriptor)?;
    if descriptor.magic != DESCRIPTOR_MAGIC {
        return Err(VolatileReadError::InvalidDescriptor);
    }
    if descriptor.address.is_null() {
        return Err(VolatileReadError::NullAddress);
    }
    let target_size = usize::try_from(target_size).map_err(|_| VolatileReadError::TypeMismatch)?;
    let target_alignment =
        usize::try_from(target_alignment).map_err(|_| VolatileReadError::TypeMismatch)?;
    if descriptor.target_size != target_size || descriptor.target_alignment != target_alignment {
        return Err(VolatileReadError::TypeMismatch);
    }
    if require_write && descriptor.writable == 0 {
        return Err(VolatileReadError::InvalidDescriptor);
    }
    if descriptor.alignment == 0 || !descriptor.alignment.is_power_of_two() {
        return Err(VolatileReadError::InvalidDescriptor);
    }
    if descriptor.target_alignment == 0
        || !descriptor.target_alignment.is_power_of_two()
        || descriptor.alignment < descriptor.target_alignment
        || !(descriptor.address as usize).is_multiple_of(descriptor.alignment)
    {
        return Err(VolatileReadError::InvalidDescriptor);
    }
    let offset = usize::try_from(offset).map_err(|_| VolatileReadError::OffsetOverflow)?;
    let end = offset.checked_add(width).ok_or(VolatileReadError::OffsetOverflow)?;
    if end > descriptor.byte_length {
        return Err(VolatileReadError::OutOfBounds);
    }
    let address = (descriptor.address as usize)
        .checked_add(offset)
        .ok_or(VolatileReadError::OffsetOverflow)?;
    if width == 0
        || target_alignment == 0
        || address % width != 0
        || address % target_alignment != 0
    {
        return Err(VolatileReadError::Misaligned);
    }
    Ok(address as *const u8)
}

fn read<T: Copy>(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
    width: usize,
    load: unsafe fn(*const T) -> T,
) -> *mut u8 {
    let result_size = if width == 8 { RESULT_U64_SIZE } else { RESULT_SCALAR_SIZE };
    let address = match validate(descriptor, offset, width, target_size, target_alignment, false) {
        Ok(address) => address,
        Err(error) => return error_result(result_size, error),
    };
    let result = allocate_result(result_size, RESULT_OK);
    if result.is_null() {
        return error_result(result_size, VolatileReadError::AllocationFailure);
    }
    let value = unsafe { load(address.cast::<T>()) };
    if width == 8 {
        let value = unsafe { std::mem::transmute_copy::<T, u64>(&value) };
        write_u64(result, U64_PAYLOAD_OFFSET, value);
    } else {
        let value = match width {
            1 => unsafe { std::mem::transmute_copy::<T, u8>(&value) as u32 },
            2 => unsafe { std::mem::transmute_copy::<T, u16>(&value) as u32 },
            4 => unsafe { std::mem::transmute_copy::<T, u32>(&value) },
            _ => return error_result(result_size, VolatileReadError::UnsupportedWidth),
        };
        write_u32(result, PAYLOAD_OFFSET, value);
    }
    result
}

fn write<T: Copy>(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
    width: usize,
    value: T,
    store: unsafe fn(*mut T, T),
) -> *mut u8 {
    let address = match validate(descriptor, offset, width, target_size, target_alignment, true) {
        Ok(address) => address,
        Err(error) => return error_result(RESULT_SCALAR_SIZE, error),
    };
    let result = allocate_result(RESULT_SCALAR_SIZE, RESULT_OK);
    if result.is_null() {
        return error_result(RESULT_SCALAR_SIZE, VolatileReadError::AllocationFailure);
    }
    unsafe { store(address.cast_mut().cast::<T>(), value) };
    result
}

/// Performs one checked volatile `u8` load.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned descriptor. The function does
/// not trust the pointer and validates its marker and layout before loading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_read_u8(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
) -> *mut u8 {
    read(descriptor, offset, target_size, target_alignment, 1, ptr::read_volatile::<u8>)
}

/// Performs one checked volatile `u16` load.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned descriptor. The function does
/// not trust the pointer and validates its marker and layout before loading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_read_u16(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
) -> *mut u8 {
    read(descriptor, offset, target_size, target_alignment, 2, ptr::read_volatile::<u16>)
}

/// Performs one checked volatile `u32` load.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned descriptor. The function does
/// not trust the pointer and validates its marker and layout before loading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_read_u32(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
) -> *mut u8 {
    read(descriptor, offset, target_size, target_alignment, 4, ptr::read_volatile::<u32>)
}

/// Performs one checked volatile `u64` load.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned descriptor. The function does
/// not trust the pointer and validates its marker and layout before loading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_read_u64(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
) -> *mut u8 {
    read(descriptor, offset, target_size, target_alignment, 8, ptr::read_volatile::<u64>)
}

/// Performs one checked volatile `u8` store.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned writable descriptor. The
/// function validates its marker, permission, and layout before storing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_write_u8(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
    value: u8,
) -> *mut u8 {
    write(descriptor, offset, target_size, target_alignment, 1, value, ptr::write_volatile::<u8>)
}

/// Performs one checked volatile `u16` store.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned writable descriptor. The
/// function validates its marker, permission, and layout before storing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_write_u16(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
    value: u16,
) -> *mut u8 {
    write(descriptor, offset, target_size, target_alignment, 2, value, ptr::write_volatile::<u16>)
}

/// Performs one checked volatile `u32` store.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned writable descriptor. The
/// function validates its marker, permission, and layout before storing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_write_u32(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
    value: u32,
) -> *mut u8 {
    write(descriptor, offset, target_size, target_alignment, 4, value, ptr::write_volatile::<u32>)
}

/// Performs one checked volatile `u64` store.
///
/// # Safety
///
/// `descriptor` must be a live provider-owned writable descriptor. The
/// function validates its marker, permission, and layout before storing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_volatile_write_u64(
    descriptor: *const VolatileAddressDescriptor,
    offset: u64,
    target_size: u64,
    target_alignment: u64,
    value: u64,
) -> *mut u8 {
    write(descriptor, offset, target_size, target_alignment, 8, value, ptr::write_volatile::<u64>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::actus_enum_drop;

    #[repr(C)]
    struct Registers {
        status: u32,
        data: u32,
    }

    #[test]
    fn validates_descriptor_and_reads_volatile_word() {
        let registers = Registers { status: 0x1122_3344, data: 0x5566_7788 };
        let descriptor = VolatileAddressDescriptor {
            magic: DESCRIPTOR_MAGIC,
            address: (&registers as *const Registers).cast(),
            byte_length: std::mem::size_of::<Registers>(),
            alignment: std::mem::align_of::<Registers>(),
            target_size: std::mem::size_of::<Registers>(),
            target_alignment: std::mem::align_of::<Registers>(),
            writable: 0,
        };
        let result = unsafe {
            actus_volatile_read_u32(
                &descriptor,
                4,
                std::mem::size_of::<Registers>() as u64,
                std::mem::align_of::<Registers>() as u64,
            )
        };
        assert!(!result.is_null());
        assert_eq!(unsafe { *(result.add(U64_PAYLOAD_OFFSET - 4).cast::<u32>()) }, 0x5566_7788);
        unsafe { actus_enum_drop(result, RESULT_SCALAR_SIZE) };
    }

    #[test]
    fn rejects_out_of_bounds_and_misaligned_reads() {
        let value = 1u32;
        let descriptor = VolatileAddressDescriptor {
            magic: DESCRIPTOR_MAGIC,
            address: (&value as *const u32).cast(),
            byte_length: 8,
            alignment: 4,
            target_size: 4,
            target_alignment: 4,
            writable: 0,
        };
        let out_of_bounds = unsafe { actus_volatile_read_u32(&descriptor, 8, 4, 4) };
        let misaligned = unsafe { actus_volatile_read_u32(&descriptor, 1, 4, 4) };
        assert!(!out_of_bounds.is_null());
        assert!(!misaligned.is_null());
        assert_eq!(unsafe { *out_of_bounds.add(PAYLOAD_OFFSET).cast::<u32>() }, 3);
        assert_eq!(unsafe { *misaligned.add(PAYLOAD_OFFSET).cast::<u32>() }, 4);
        unsafe {
            actus_enum_drop(out_of_bounds, RESULT_SCALAR_SIZE);
            actus_enum_drop(misaligned, RESULT_SCALAR_SIZE);
        }
    }

    #[test]
    fn writable_descriptor_allows_store_and_read_only_descriptor_rejects_it() {
        let mut value = 1u32;
        let descriptor = VolatileAddressDescriptor {
            magic: DESCRIPTOR_MAGIC,
            address: (&mut value as *mut u32).cast(),
            byte_length: std::mem::size_of::<u32>(),
            alignment: std::mem::align_of::<u32>(),
            target_size: std::mem::size_of::<u32>(),
            target_alignment: std::mem::align_of::<u32>(),
            writable: 1,
        };
        let result = unsafe { actus_volatile_write_u32(&descriptor, 0, 4, 4, 9) };
        assert!(!result.is_null());
        assert_eq!(value, 9);
        unsafe { actus_enum_drop(result, RESULT_SCALAR_SIZE) };

        let read_only = VolatileAddressDescriptor { writable: 0, ..descriptor };
        let rejected = unsafe { actus_volatile_write_u32(&read_only, 0, 4, 4, 10) };
        assert!(!rejected.is_null());
        assert_eq!(value, 9);
        assert_eq!(unsafe { *(rejected.add(PAYLOAD_OFFSET).cast::<u32>()) }, 0);
        unsafe { actus_enum_drop(rejected, RESULT_SCALAR_SIZE) };
    }

    #[test]
    fn secure_zero_overwrites_the_complete_owned_storage() {
        let mut secret = [0xA5u8; 32];
        unsafe { crate::runtime::actus_secure_zero(secret.as_mut_ptr(), secret.len()) };
        assert_eq!(secret, [0u8; 32]);
    }
}

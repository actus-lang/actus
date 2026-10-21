use super::contract::{
    PROCESS_MAX_ARGUMENT_BYTES, PROCESS_MAX_ARGUMENTS, PROCESS_STATUS_ARGUMENT_TOO_LARGE,
    PROCESS_STATUS_BUFFER_TOO_SMALL, PROCESS_STATUS_INVALID_ENCODING, PROCESS_STATUS_INVALID_INDEX,
    PROCESS_STATUS_TOO_MANY_ARGUMENTS, PROCESS_STATUS_UNAVAILABLE,
};
use super::types::{BufferHandle, restore_buffer};

fn collect_arguments() -> Result<Vec<Vec<u8>>, i32> {
    let mut arguments = Vec::new();
    for argument in std::env::args_os() {
        if arguments.len() >= PROCESS_MAX_ARGUMENTS {
            return Err(PROCESS_STATUS_TOO_MANY_ARGUMENTS);
        }
        let argument = argument.into_string().map_err(|_| PROCESS_STATUS_INVALID_ENCODING)?;
        if argument.len() > PROCESS_MAX_ARGUMENT_BYTES {
            return Err(PROCESS_STATUS_ARGUMENT_TOO_LARGE);
        }
        arguments.push(argument.into_bytes());
    }
    Ok(arguments)
}

/// Return the bounded hosted process argument count, including argv[0].
#[unsafe(no_mangle)]
pub extern "C" fn actus_process_arg_count() -> i32 {
    match collect_arguments() {
        Ok(arguments) => i32::try_from(arguments.len()).unwrap_or(PROCESS_STATUS_UNAVAILABLE),
        Err(status) => status,
    }
}

/// Copy one hosted process argument into a caller-owned buffer from offset zero.
///
/// # Safety
///
/// `handle` must be null or point to a live buffer allocation owned by the
/// caller for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_process_arg(handle: BufferHandle, index: i32) -> i32 {
    if handle.is_null() {
        return PROCESS_STATUS_UNAVAILABLE;
    }
    if index < 0 {
        return PROCESS_STATUS_INVALID_INDEX;
    }
    let buffer = unsafe { &mut *handle };
    if buffer.length > buffer.capacity || (buffer.length > 0 && buffer.data.is_null()) {
        return PROCESS_STATUS_UNAVAILABLE;
    }
    let mut data = unsafe { Vec::from_raw_parts(buffer.data, buffer.length, buffer.capacity) };
    let result = match collect_arguments() {
        Err(status) => Err(status),
        Ok(arguments) => match arguments.get(index as usize) {
            None => Err(PROCESS_STATUS_INVALID_INDEX),
            Some(argument) if argument.len() > data.capacity() => {
                Err(PROCESS_STATUS_BUFFER_TOO_SMALL)
            }
            Some(argument) => {
                data.clear();
                data.extend_from_slice(argument);
                Ok(i32::try_from(argument.len()).unwrap_or(PROCESS_STATUS_UNAVAILABLE))
            }
        },
    };
    match result {
        Ok(status) => {
            restore_buffer(buffer, data);
            status
        }
        Err(status) => {
            restore_buffer(buffer, data);
            status
        }
    }
}

use actus::runtime::{ActusBuffer, actus_buffer_allocate, actus_buffer_append, actus_buffer_drop};

#[test]
fn reports_success_after_printing_an_integer() {
    assert_eq!(actus::runtime::actus_print_int(42), 0);
}

#[test]
fn returns_printed_string_length() {
    let text = std::ffi::CString::new("Actus").expect("test string should be valid");
    let length = unsafe { actus::runtime::actus_print_string(text.as_ptr().cast()) };
    assert_eq!(length, 5);
}

#[test]
fn validates_borrowed_string_bytes_without_allocation() {
    let text = std::ffi::CString::new("é").expect("string should be valid");
    let pointer = text.as_ptr().cast();
    assert_eq!(unsafe { actus::runtime::actus_string_length(pointer) }, 2);
    assert_eq!(unsafe { actus::runtime::actus_string_byte_at(pointer, 0) }, 0xc3);
    assert_eq!(unsafe { actus::runtime::actus_string_byte_at(pointer, 1) }, 0xa9);
    assert_eq!(
        unsafe { actus::runtime::actus_string_byte_at(pointer, 2) },
        actus::runtime::STRING_STATUS_OUT_OF_BOUNDS
    );
}

#[test]
fn rejects_null_and_invalid_utf8_string_inputs() {
    assert_eq!(
        unsafe { actus::runtime::actus_string_length(std::ptr::null()) },
        actus::runtime::STRING_STATUS_NULL
    );
    let invalid = [0xff, 0];
    assert_eq!(
        unsafe { actus::runtime::actus_string_length(invalid.as_ptr()) },
        actus::runtime::STRING_STATUS_INVALID_UTF8
    );
}

#[test]
fn copies_string_bytes_into_caller_owned_storage_with_capacity_status() {
    let text = std::ffi::CString::new("Actus").expect("string should be valid");
    let storage = actus_buffer_allocate(0);
    assert!(!storage.is_null());
    unsafe { assert_eq!(actus::runtime::actus_buffer_reserve(storage, 5), 5) };
    let copied =
        unsafe { actus::runtime::actus_string_copy_to_buffer(text.as_ptr().cast(), storage) };
    assert_eq!(copied, 5);
    unsafe {
        assert_eq!((*storage).length, 5);
        assert_eq!(std::slice::from_raw_parts((*storage).data, 5), b"Actus");
        actus_buffer_drop(storage);
    }

    let small = actus_buffer_allocate(0);
    assert!(!small.is_null());
    unsafe { assert_eq!(actus::runtime::actus_buffer_reserve(small, 4), 4) };
    assert_eq!(
        unsafe { actus::runtime::actus_string_copy_to_buffer(text.as_ptr().cast(), small) },
        actus::runtime::STRING_COPY_STATUS_CAPACITY
    );
    unsafe { actus_buffer_drop(small) };
}

#[test]
fn prints_exact_buffer_length_without_a_null_terminator() {
    let mut storage = [b'A', 0, b'Z'];
    let mut buffer =
        ActusBuffer { data: storage.as_mut_ptr(), length: storage.len(), capacity: storage.len() };
    let count = unsafe { actus::runtime::actus_print_buffer_stdout(&mut buffer) };
    assert_eq!(count, 3);
}

#[test]
fn allocates_appends_and_drops_a_buffer() {
    let handle = actus_buffer_allocate(4);
    assert!(!handle.is_null());

    unsafe {
        assert_eq!((*handle).length, 4);
        assert!(actus_buffer_append(handle, 42));
        assert_eq!((*handle).length, 5);
        actus_buffer_drop(handle);
    }
}

#[test]
fn rejects_null_buffer_operations_without_memory_access() {
    unsafe { actus_buffer_drop(std::ptr::null_mut()) };
    assert!(!unsafe { actus_buffer_append(std::ptr::null_mut(), 42) });
}

#[test]
fn preserves_the_c_layout_fields_in_declaration_order() {
    assert_eq!(std::mem::offset_of!(ActusBuffer, data), 0);
    assert_eq!(std::mem::offset_of!(ActusBuffer, length), std::mem::size_of::<*mut u8>());
    assert_eq!(
        std::mem::offset_of!(ActusBuffer, capacity),
        std::mem::size_of::<*mut u8>() + std::mem::size_of::<usize>()
    );
}

#[test]
fn exposes_the_cargo_built_runtime_archive() {
    let archive = actus::runtime::runtime_archive_path().expect("runtime archive should exist");
    assert!(archive.is_file(), "runtime archive path should point to a file");
}

#[test]
fn exposes_the_versioned_runtime_contract_symbols() {
    assert_eq!(actus::runtime::RUNTIME_ABI_VERSION, 1);
    assert_eq!(actus::runtime::ABI_STATUS_SUCCESS, 0);
    assert_eq!(actus::runtime::ABI_STATUS_FAILURE, -1);
    assert_eq!(actus::runtime::ABI_STATUS_END_OF_STREAM, -2);
    assert_eq!(actus::runtime::ABI_HANDLE_FAILURE, -1);
    assert!(actus::runtime::is_successful_count(0));
    assert!(actus::runtime::is_successful_count(37));
    assert!(!actus::runtime::is_successful_count(-1));
    assert_eq!(actus::runtime::BUFFER_ALLOCATE_SYMBOL, "actus_buffer_allocate");
    assert_eq!(actus::runtime::BUFFER_APPEND_SYMBOL, "actus_buffer_append");
    assert_eq!(actus::runtime::BUFFER_DROP_SYMBOL, "actus_buffer_drop");
    assert_eq!(actus::runtime::MONOTONIC_NANOS_SYMBOL, "actus_monotonic_nanos");
}

#[test]
fn hosted_monotonic_clock_reads_are_non_decreasing() {
    let first = actus::runtime::actus_monotonic_nanos();
    let second = actus::runtime::actus_monotonic_nanos();
    assert!(second >= first);
}

#[test]
fn invalid_file_bridge_inputs_use_the_documented_failure_status() {
    assert_eq!(actus::runtime::actus_file_flush(-1), actus::runtime::ABI_STATUS_FAILURE);
    assert_eq!(actus::runtime::actus_file_close(-1), actus::runtime::ABI_STATUS_FAILURE);
    assert_eq!(
        actus::runtime::actus_file_seek_buffer(-1, 0, 0),
        actus::runtime::ABI_STATUS_FAILURE
    );
}

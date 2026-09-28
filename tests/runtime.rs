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

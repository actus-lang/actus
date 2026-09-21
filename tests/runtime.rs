use actus::runtime::{ActusBuffer, actus_buffer_allocate, actus_buffer_append, actus_buffer_drop};

#[test]
fn returns_printed_integer_value() {
    assert_eq!(actus::runtime::actus_print_int(42), 42);
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

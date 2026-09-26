use actus::runtime::{
    actus_buffer_allocate, actus_buffer_append, actus_buffer_reserve, actus_buffered_write_stdout,
    actus_flush_buffered_stdout,
};

#[test]
fn std_io_buffered_runtime_flushes_full_and_partial_chunks() {
    let target = actus_buffer_allocate(0);
    let source = actus_buffer_allocate(0);
    assert!(!target.is_null());
    assert!(!source.is_null());
    unsafe {
        assert_eq!(actus_buffer_reserve(target, 4), 4);
        for byte in b"abcdef" {
            assert!(actus_buffer_append(source, *byte));
        }
        assert_eq!(actus_buffered_write_stdout(target, source), 6);
        assert_eq!(actus_flush_buffered_stdout(target), 2);
        actus::runtime::actus_buffer_drop(source);
        actus::runtime::actus_buffer_drop(target);
    }
}

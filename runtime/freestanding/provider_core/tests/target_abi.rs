use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::sync::atomic::{AtomicUsize, Ordering};

use actus_freestanding_region_provider::{
    ActusBuffer, actus_region_cancel, actus_region_close, actus_region_drop, actus_region_open,
    actus_region_publish, actus_region_read, actus_region_write,
};

static BUFFER_RELEASES: AtomicUsize = AtomicUsize::new(0usize);

#[unsafe(no_mangle)]
extern "C" fn actus_enum_allocate(size: usize) -> *mut u8 {
    let Ok(layout) = Layout::from_size_align(size, 8usize) else {
        return std::ptr::null_mut();
    };
    unsafe { alloc_zeroed(layout) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn actus_enum_drop(pointer: *mut u8, size: usize) {
    if pointer.is_null() {
        return;
    }
    let Ok(layout) = Layout::from_size_align(size, 8usize) else {
        return;
    };
    unsafe { dealloc(pointer, layout) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn actus_buffer_drop(_handle: *mut ActusBuffer) {
    BUFFER_RELEASES.fetch_add(1usize, Ordering::SeqCst);
}

unsafe fn result_discriminant(result: *mut u8) -> u32 {
    unsafe { std::ptr::read_unaligned(result.cast::<u32>()) }
}

#[test]
fn target_abi_links_and_runs_region_lifecycle() {
    BUFFER_RELEASES.store(0usize, Ordering::SeqCst);
    let mut backing_storage = [0u8; 1024];
    let mut backing = ActusBuffer {
        data: backing_storage.as_mut_ptr(),
        length: backing_storage.len(),
        capacity: backing_storage.len(),
    };
    let open_result = unsafe { actus_region_open(&mut backing, 1u64, 1024u64, 0u64, 1024u64) };
    assert!(!open_result.is_null());
    assert_eq!(unsafe { result_discriminant(open_result) }, 0u32);
    assert_eq!(BUFFER_RELEASES.load(Ordering::SeqCst), 1usize);
    let descriptor = unsafe { std::ptr::read_unaligned(open_result.add(8usize).cast::<*mut u8>()) };
    assert!(!descriptor.is_null());
    unsafe { actus_enum_drop(open_result, 64usize) };

    let mut source_storage = [42u8; 1];
    let source =
        ActusBuffer { data: source_storage.as_mut_ptr(), length: 1usize, capacity: 1usize };
    let write_result = unsafe { actus_region_write(descriptor, 7u64, &source) };
    assert_eq!(unsafe { result_discriminant(write_result) }, 0u32);
    unsafe { actus_enum_drop(write_result, 8usize) };

    let publish_result = unsafe { actus_region_publish(descriptor) };
    assert_eq!(unsafe { result_discriminant(publish_result) }, 0u32);
    unsafe { actus_enum_drop(publish_result, 16usize) };

    let mut destination_storage = [0u8; 1];
    let mut destination =
        ActusBuffer { data: destination_storage.as_mut_ptr(), length: 1usize, capacity: 1usize };
    let read_result = unsafe { actus_region_read(descriptor, 7u64, &mut destination) };
    assert_eq!(unsafe { result_discriminant(read_result) }, 0u32);
    assert_eq!(destination_storage, [42u8]);
    unsafe { actus_enum_drop(read_result, 8usize) };

    let cancel_result = unsafe { actus_region_cancel(descriptor) };
    assert_eq!(unsafe { result_discriminant(cancel_result) }, 0u32);
    unsafe { actus_enum_drop(cancel_result, 8usize) };

    let close_result = unsafe { actus_region_close(descriptor) };
    assert_eq!(unsafe { result_discriminant(close_result) }, 0u32);
    unsafe { actus_enum_drop(close_result, 8usize) };
    assert_eq!(unsafe { actus_region_drop(0x0000_0001_0000_0001u64) }, 0i32);
}

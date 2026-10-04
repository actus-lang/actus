use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use actus::runtime::{
    ActusBuffer, ActusPath, ActusPathCView, ActusPathComponent, ActusPathComponents,
    actus_path_c_view, actus_path_components, actus_path_extension, actus_path_file_name,
    actus_path_is_absolute, actus_path_next_component, actus_path_parent,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn borrowed_path_inspection_does_not_allocate() {
    let mut storage = b"/tmp/archive.tar.gz\0".to_vec();
    let mut buffer = ActusBuffer {
        data: storage.as_mut_ptr(),
        length: storage.len(),
        capacity: storage.capacity(),
    };
    let path = ActusPath {
        storage: &mut buffer,
        platform: 0,
        length: 19,
        capacity: storage.capacity() as i32,
        terminated: 1,
    };
    let mut component = ActusPathComponent { source: std::ptr::null(), offset: 0, length: 0 };
    let mut iterator = ActusPathComponents { source: std::ptr::null(), offset: 0, limit: 0 };
    let mut view = ActusPathCView { data: std::ptr::null(), length: 0 };

    // LLVM coverage initializes instrumentation lazily on the first call into
    // an instrumented function. Warm that one-time test-process overhead before
    // measuring the runtime contract; allocations made by the path API on each
    // call still change the counter and fail the assertion below.
    #[cfg(coverage)]
    unsafe {
        let _ = actus_path_is_absolute(&path);
        let _ = actus_path_parent(&mut component, &path);
        let _ = actus_path_file_name(&mut component, &path);
        let _ = actus_path_extension(&mut component, &path);
        actus_path_components(&mut iterator, &path);
        let _ = actus_path_next_component(&mut component, &mut iterator);
        let _ = actus_path_c_view(&mut view, &path);
    }

    let before = ALLOCATIONS.load(Ordering::Relaxed);

    unsafe {
        assert_eq!(actus_path_is_absolute(&path), 1);
        assert!(!actus_path_parent(&mut component, &path).is_null());
        assert!(!actus_path_file_name(&mut component, &path).is_null());
        assert!(!actus_path_extension(&mut component, &path).is_null());
        actus_path_components(&mut iterator, &path);
        assert!(!actus_path_next_component(&mut component, &mut iterator).is_null());
        assert!(!actus_path_c_view(&mut view, &path).is_null());
    }

    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), before);
    assert_eq!(view.length, 19);
    assert_eq!(storage.last(), Some(&0));
}

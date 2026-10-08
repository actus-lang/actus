use super::actus_region_drop;
use super::region::RegionError;
use super::region_bridge::REGION_STORE;

#[test]
fn native_drop_releases_a_descriptor_once() {
    let descriptor = {
        let mut store = REGION_STORE.lock().expect("region store should not be poisoned");
        let descriptor =
            store.allocate(vec![0u8; 4], 4, 1, 1, 0, 1).expect("region slot should allocate");
        let handle = descriptor.handle;
        let pointer = Box::into_raw(Box::new(descriptor));
        store.attach_descriptor(handle, pointer).expect("descriptor should attach");
        pointer
    };

    assert_eq!(unsafe { actus_region_drop(descriptor.cast()) }, 0);
    assert_eq!(unsafe { actus_region_drop(descriptor.cast()) }, -1);
}

#[test]
fn failed_close_preserves_the_owner_and_rejects_repeated_close() {
    let (handle, pointer, generation) = {
        let mut store = REGION_STORE.lock().expect("region store should not be poisoned");
        let descriptor =
            store.allocate(vec![0u8; 4], 4, 1, 1, 0, 1).expect("region slot should allocate");
        let handle = descriptor.handle;
        let generation = descriptor.generation;
        let pointer = Box::into_raw(Box::new(descriptor));
        store.attach_descriptor(handle, pointer).expect("descriptor should attach");
        (handle, pointer, generation)
    };

    {
        let mut store = REGION_STORE.lock().expect("region store should not be poisoned");
        assert_eq!(
            store.close(handle, generation + 1, pointer as usize),
            Err(RegionError::StaleGeneration)
        );
        assert!(store.get_mut(handle).is_ok());
        store.close(handle, generation, pointer as usize).expect("live close should succeed");
        assert_eq!(
            store.close(handle, generation, pointer as usize),
            Err(RegionError::InvalidHandle)
        );
    }

    assert_eq!(unsafe { actus_region_drop(pointer as *mut u8) }, 0);
}

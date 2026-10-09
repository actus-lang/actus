use std::mem::{align_of, offset_of, size_of};

use super::{InMemoryRegion, RegionAddressProfile, RegionDescriptor, RegionError, copy_bulk_bytes};

#[test]
fn bounded_views_read_write_and_publish() {
    let mut region = InMemoryRegion::new(7, 2, 8, 2, 3).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write(3, &[41, 42]).expect("resident element should be writable");
    }
    assert_eq!(region.descriptor().dirty, 1);
    let next = region.publish(generation).expect("publication should advance generation");
    let view = region.borrow_abs(7, next).expect("read view should open");
    let mut value = [0u8; 2];
    view.read(3, &mut value).expect("resident element should be readable");
    assert_eq!(value, [41, 42]);
}

#[test]
fn bounded_range_access_is_atomic_and_supports_empty_and_full_windows() {
    let mut region = InMemoryRegion::new(7, 2, 8, 2, 3).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        assert_eq!(view.write_range(2, 3, &[1, 2, 3, 4, 5, 6]), Ok(3));
        assert_eq!(view.write_range(2, 3, &[9]), Err(RegionError::BufferTooSmall));
        assert_eq!(
            view.write_range(4, 2, &[7, 8, 9, 10]),
            Err(RegionError::LogicalIndexOutOfBounds)
        );
        assert_eq!(view.write_range(u64::MAX, 1, &[]), Err(RegionError::OffsetOverflow));
    }
    assert_eq!(region.descriptor().dirty, 1);
    let next = region.publish(generation).expect("publication should advance generation");
    let view = region.borrow_abs(7, next).expect("read view should open");
    let mut value = [0u8; 6];
    assert_eq!(view.read_range(2, 3, &mut value), Ok(3));
    assert_eq!(value, [1, 2, 3, 4, 5, 6]);
    let mut empty = [];
    assert_eq!(view.read_range(8, 0, &mut empty), Ok(0));
}

#[test]
fn failed_range_validation_preserves_buffers_and_region_state() {
    let mut region = InMemoryRegion::new(7, 2, 4, 0, 2).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write_range(0, 2, &[1, 2, 3, 4]).expect("valid range should be writable");
    }
    let staged = region.storage.clone();
    let dirty = region.descriptor().dirty;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        assert_eq!(view.write_range(0, 2, &[9]), Err(RegionError::BufferTooSmall));
    }
    assert_eq!(region.storage, staged);
    assert_eq!(region.descriptor().dirty, dirty);
    let view = region.borrow_abs(7, generation).expect("read view should open");
    let mut destination = [9u8; 1];
    assert_eq!(view.read_range(0, 2, &mut destination), Err(RegionError::BufferTooSmall));
    assert_eq!(destination, [9]);
}

#[test]
fn bounded_ranges_reject_payloads_above_the_bulk_limit() {
    let mut region = InMemoryRegion::new(7, 1, 131_073, 0, 131_073)
        .expect("resident window should fit the bounded test payload");
    let generation = region.descriptor().generation;
    let view = region.borrow_abs(7, generation).expect("read view should open");
    let mut accepted = vec![0u8; 131_072];
    assert_eq!(view.read_range(0, 131_072, &mut accepted), Ok(131_072));
    let mut rejected = vec![0u8; 131_073];
    assert_eq!(view.read_range(0, 131_073, &mut rejected), Err(RegionError::BulkLimitExceeded));
}

#[test]
fn target_profile_can_lower_the_bulk_limit_without_changing_region_operations() {
    let profile = RegionAddressProfile::new(32, 32)
        .expect("32-bit profile should be valid")
        .with_bulk_limit(64)
        .expect("lower bulk limit should be valid");
    let mut region = InMemoryRegion::new_with_profile(profile, 7, 1, 1, 128, 0, 128)
        .expect("lower-limit region should be valid");
    let generation = region.descriptor().generation;
    let view = region.borrow_abs(7, generation).expect("lower-limit read view should open");
    let mut accepted = vec![0u8; 64];
    assert_eq!(view.read_range(0, 64, &mut accepted), Ok(64));
    let mut rejected = vec![0u8; 65];
    assert_eq!(view.read_range(0, 65, &mut rejected), Err(RegionError::BulkLimitExceeded));
}

#[test]
fn aggregate_bytes_are_preserved_across_pointer_width_profiles() {
    for (index_bits, offset_bits) in [(32u8, 32u8), (64u8, 64u8)] {
        let profile = RegionAddressProfile::new(index_bits, offset_bits)
            .expect("supported pointer profiles should be valid");
        let mut region = InMemoryRegion::new_with_profile(profile, 7, 4, 2, 2, 0, 2)
            .expect("profiled Region should be valid");
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
            view.write_range(0, 2, &[0x01, 0x02, 0x80, 0xff, 0x10, 0x20, 0x40, 0x7f])
                .expect("aggregate bytes should be writable");
        }
        let next = region.publish(generation).expect("publication should advance generation");
        let view = region.borrow_abs(7, next).expect("read view should open");
        let mut destination = [0u8; 8];
        view.read_range(0, 2, &mut destination).expect("aggregate bytes should be readable");
        assert_eq!(destination, [0x01, 0x02, 0x80, 0xff, 0x10, 0x20, 0x40, 0x7f]);
    }
}

#[test]
fn bulk_copy_preserves_memmove_overlap_semantics() {
    let mut bytes = [1u8, 2, 3, 4];
    unsafe { copy_bulk_bytes(bytes.as_ptr(), bytes.as_mut_ptr().add(1), 3) };
    assert_eq!(bytes, [1, 1, 2, 3]);
}

#[test]
fn stale_and_out_of_window_accesses_are_rejected() {
    let mut region = InMemoryRegion::new(7, 2, 8, 2, 3).expect("region should be valid");
    let generation = region.descriptor().generation;
    assert!(matches!(region.borrow_abs(8, generation), Err(RegionError::InvalidHandle)));
    assert_eq!(region.borrow_abs(7, generation + 1).unwrap_err(), RegionError::StaleGeneration);
    let view = region.borrow_abs(7, generation).expect("read view should open");
    let mut value = [0u8; 2];
    assert_eq!(view.read(1, &mut value), Err(RegionError::LogicalIndexOutOfBounds));
    assert_eq!(view.read(2, &mut [0u8; 1]), Err(RegionError::BufferTooSmall));
}

#[test]
fn rejects_invalid_windows_and_generation_exhaustion() {
    assert!(matches!(InMemoryRegion::new(7, 2, 8, 7, 2), Err(RegionError::InvalidWindow)));
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    region.descriptor.generation = u64::MAX;
    assert_eq!(region.publish(u64::MAX), Err(RegionError::GenerationExhausted));
}

#[test]
fn cancellation_restores_the_last_published_window() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write(0, &[41, 42]).expect("resident element should be writable");
    }
    region.cancel(generation).expect("cancellation should succeed");
    let view = region.borrow_abs(7, generation).expect("read view should open");
    let mut value = [9u8; 2];
    view.read(0, &mut value).expect("resident element should be readable");
    assert_eq!(value, [0, 0]);
    assert_eq!(region.descriptor().dirty, 0);
}

#[test]
fn publication_commits_all_staged_elements_as_one_bounded_transaction() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 2).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write_range(0, 2, &[1, 2, 3, 4]).expect("both elements should be staged");
    }
    let next = region.publish(generation).expect("publication should accept the transaction");
    assert_eq!(next, generation + 1);
    let view = region.borrow_abs(7, next).expect("published view should open");
    let mut value = [0u8; 4];
    view.read_range(0, 2, &mut value).expect("published range should be readable");
    assert_eq!(value, [1, 2, 3, 4]);
    assert_eq!(region.descriptor().dirty, 0);
}

#[test]
fn repeated_publish_and_cancel_follow_the_same_generation_contract() {
    let mut region = InMemoryRegion::new(7, 1, 4, 0, 1).expect("region should be valid");
    let first_generation = region.descriptor().generation;
    {
        let mut view =
            region.borrow_ins(7, first_generation).expect("first mutable view should open");
        view.write(0, &[31]).expect("first value should be staged");
    }
    let second_generation =
        region.publish(first_generation).expect("first publication should succeed");
    {
        let mut view =
            region.borrow_ins(7, second_generation).expect("second mutable view should open");
        view.write(0, &[32]).expect("second value should be staged");
    }
    let third_generation =
        region.publish(second_generation).expect("second publication should succeed");
    assert_eq!(third_generation, second_generation + 1);
    region.cancel(third_generation).expect("clean cancellation should be idempotent");
    region.cancel(third_generation).expect("repeated cancellation should be safe");
    assert_eq!(region.descriptor().generation, third_generation);
    let view = region.borrow_abs(7, third_generation).expect("latest snapshot should open");
    let mut value = [0u8; 1];
    view.read(0, &mut value).expect("latest value should be readable");
    assert_eq!(value, [32]);
}

#[test]
fn cancellation_discards_all_staged_elements_without_advancing_generation() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 2).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write_range(0, 2, &[5, 6, 7, 8]).expect("both elements should be staged");
    }
    region.cancel(generation).expect("cancellation should restore the snapshot");
    assert_eq!(region.descriptor().generation, generation);
    let view = region.borrow_abs(7, generation).expect("restored view should open");
    let mut value = [9u8; 4];
    view.read_range(0, 2, &mut value).expect("restored range should be readable");
    assert_eq!(value, [0, 0, 0, 0]);
    assert_eq!(region.descriptor().dirty, 0);
}

#[test]
fn failed_publication_preserves_dirty_staged_bytes_for_retry() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 2).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write_range(0, 2, &[11, 12, 13, 14]).expect("both elements should be staged");
    }
    assert_eq!(region.publish(generation + 1), Err(RegionError::StaleGeneration));
    assert_eq!(region.descriptor().generation, generation);
    assert_eq!(region.descriptor().dirty, 1);
    let next = region.publish(generation).expect("retry with the live generation should work");
    assert_eq!(next, generation + 1);
    let view = region.borrow_abs(7, next).expect("retried publication should be readable");
    let mut value = [0u8; 4];
    view.read_range(0, 2, &mut value).expect("retried range should be readable");
    assert_eq!(value, [11, 12, 13, 14]);
}

#[test]
fn generation_exhaustion_preserves_dirty_staged_bytes() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write(0, &[21, 22]).expect("element should be staged");
    }
    region.descriptor.generation = u64::MAX;
    assert_eq!(region.publish(u64::MAX), Err(RegionError::GenerationExhausted));
    assert_eq!(region.descriptor().generation, u64::MAX);
    assert_eq!(region.descriptor().dirty, 1);
    let view = region.borrow_abs(7, u64::MAX).expect("failed publication must keep access live");
    let mut value = [0u8; 2];
    view.read(0, &mut value).expect("staged bytes should remain readable");
    assert_eq!(value, [21, 22]);
}

#[test]
fn invalid_publication_storage_preserves_the_live_transaction() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write(0, &[41, 42]).expect("element should be staged");
    }
    region.storage.pop();
    assert_eq!(region.publish(generation), Err(RegionError::InvalidDescriptor));
    assert_eq!(region.descriptor().generation, generation);
    assert_eq!(region.descriptor().dirty, 1);
    assert_eq!(region.published_storage, [0, 0]);
}

#[test]
fn failed_publication_preserves_the_current_generation() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let generation = region.descriptor().generation;
    assert_eq!(region.publish(generation + 1), Err(RegionError::StaleGeneration));
    assert_eq!(region.descriptor().generation, generation);
}

#[test]
fn remap_replaces_a_clean_window_and_advances_generation() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let generation = region.descriptor().generation;
    let next =
        region.remap(generation, vec![41, 42, 43, 44], 4, 2).expect("clean window should remap");
    let descriptor = region.descriptor();
    assert_eq!(descriptor.window_start, 4);
    assert_eq!(descriptor.window_count, 2);
    assert_eq!(descriptor.generation, next);
    let view = region.borrow_abs(7, next).expect("new window should open");
    let mut value = [0u8; 2];
    view.read(5, &mut value).expect("new resident element should be readable");
    assert_eq!(value, [43, 44]);
}

#[test]
fn remap_rejects_dirty_and_invalid_windows_without_changing_state() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write(0, &[9, 8]).expect("resident element should be writable");
    }
    assert_eq!(region.remap(generation, vec![1, 2], 2, 1), Err(RegionError::WindowBusy));
    let descriptor = region.descriptor();
    assert_eq!(descriptor.window_start, 0);
    assert_eq!(descriptor.window_count, 1);
    assert_eq!(descriptor.generation, generation);
    let mut clean = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let clean_generation = clean.descriptor().generation;
    assert_eq!(clean.remap(clean_generation, vec![1, 2], 8, 1), Err(RegionError::InvalidWindow));
    assert_eq!(clean.descriptor().window_start, 0);
}

#[test]
fn address_profiles_reject_unsupported_widths_and_large_32_bit_regions() {
    assert_eq!(RegionAddressProfile::new(16, 32), Err(RegionError::UnsupportedAddressWidth));
    let profile = RegionAddressProfile::new(32, 32).expect("32-bit profile should be valid");
    let maximum_length = u64::from(u32::MAX) + 1;
    assert!(InMemoryRegion::new_with_profile(profile, 7, 1, 1, maximum_length, 0, 1).is_ok());
    assert!(matches!(
        InMemoryRegion::new_with_profile(profile, 7, 1, 1, maximum_length + 1, 0, 1),
        Err(RegionError::OffsetOverflow)
    ));
    assert!(matches!(
        InMemoryRegion::new_with_profile(profile, 7, 2, 1, maximum_length, 0, 1),
        Err(RegionError::OffsetOverflow)
    ));
}

#[test]
fn wide_logical_capacity_allocates_only_the_resident_window() {
    let profile = RegionAddressProfile::new(64, 64).expect("64-bit profile should be valid");
    let logical_length = u64::from(u32::MAX) + 2;
    let mut region = InMemoryRegion::new_with_profile(profile, 7, 4, 4, logical_length, 99, 1)
        .expect("large logical capacity should fit a small resident window");
    let generation = region.descriptor().generation;
    let view = region.borrow_abs(7, generation).expect("resident window should open");
    let mut value = [0u8; 4];
    view.read(99, &mut value).expect("resident element should be readable");
    assert_eq!(value, [0, 0, 0, 0]);
}

#[test]
fn page_crossing_last_element_and_window_boundary_are_checked() {
    let mut region =
        InMemoryRegion::new(7, 2, 8194, 4095, 2).expect("cross-page window should be valid");
    let generation = region.descriptor().generation;
    {
        let mut view = region.borrow_ins(7, generation).expect("mutable view should open");
        view.write(4096, &[41, 42]).expect("last resident element should be writable");
    }
    let view = region.borrow_abs(7, generation).expect("read view should open");
    let mut value = [0u8; 2];
    view.read(4096, &mut value).expect("last resident element should be readable");
    assert_eq!(value, [41, 42]);
    assert_eq!(view.read(4097, &mut value), Err(RegionError::LogicalIndexOutOfBounds));
}

#[test]
fn descriptor_uses_fixed_width_c_layout() {
    assert_eq!(size_of::<RegionDescriptor>(), RegionDescriptor::abi_size());
    assert_eq!(align_of::<RegionDescriptor>(), RegionDescriptor::abi_alignment());
    assert_eq!(RegionDescriptor::abi_size(), 64);
    assert_eq!(RegionDescriptor::abi_alignment(), 8);
    assert_eq!(RegionDescriptor::abi_version(), 3);
    assert_eq!(offset_of!(RegionDescriptor, handle), 0);
    assert_eq!(offset_of!(RegionDescriptor, element_stride), 8);
    assert_eq!(offset_of!(RegionDescriptor, element_alignment), 16);
    assert_eq!(offset_of!(RegionDescriptor, logical_length), 24);
    assert_eq!(offset_of!(RegionDescriptor, window_start), 32);
    assert_eq!(offset_of!(RegionDescriptor, window_count), 40);
    assert_eq!(offset_of!(RegionDescriptor, generation), 48);
    assert_eq!(offset_of!(RegionDescriptor, dirty), 56);
    assert_eq!(offset_of!(RegionDescriptor, logical_index_bits), 57);
    assert_eq!(offset_of!(RegionDescriptor, byte_offset_bits), 58);
    assert_eq!(offset_of!(RegionDescriptor, pinned), 59);
}

#[test]
fn pinning_blocks_remap_but_allows_publication_and_unpin_is_idempotent() {
    let mut region = InMemoryRegion::new(7, 2, 8, 0, 1).expect("region should be valid");
    let generation = region.descriptor().generation;
    region.pin(generation).expect("pin should succeed");
    assert_eq!(region.descriptor().pinned, 1);
    assert_eq!(region.remap(generation, vec![1, 2], 2, 1), Err(RegionError::WindowBusy));
    let next = region.publish(generation).expect("publication should remain allowed while pinned");
    region.unpin(next).expect("unpin should succeed");
    region.unpin(next).expect("repeated unpin should be idempotent");
    assert_eq!(region.descriptor().pinned, 0);
    region.remap(next, vec![3, 4], 2, 1).expect("unpin should restore remap capability");
}

#[test]
fn invalid_pin_state_fails_descriptor_validation() {
    let valid = InMemoryRegion::new(7, 1, 1, 0, 1)
        .expect("valid region should provide a descriptor")
        .descriptor();
    assert_eq!(
        RegionDescriptor { pinned: 2, ..valid }.validate(),
        Err(RegionError::InvalidDescriptor)
    );
}

#[test]
fn invalid_descriptors_fail_through_typed_validation() {
    let valid = InMemoryRegion::new(7, 2, 8, 0, 1)
        .expect("valid region should provide a descriptor")
        .descriptor();
    let invalid_descriptors = [
        RegionDescriptor { handle: 0, ..valid },
        RegionDescriptor { element_stride: 0, ..valid },
        RegionDescriptor { element_alignment: 0, ..valid },
        RegionDescriptor { element_alignment: 3, ..valid },
        RegionDescriptor { element_alignment: 16, ..valid },
        RegionDescriptor { logical_length: 0, ..valid },
        RegionDescriptor { window_count: 0, ..valid },
        RegionDescriptor { generation: 0, ..valid },
        RegionDescriptor { logical_index_bits: 16, ..valid },
        RegionDescriptor { logical_length: u64::MAX, ..valid },
    ];
    for descriptor in invalid_descriptors {
        assert!(descriptor.validate().is_err());
    }
}

#[test]
fn deterministic_boundary_matrix_preserves_region_failure_classes() {
    let cases = [
        (1u64, 0u64, 1u64, 0u64),
        (4u64, 0u64, 2u64, 1u64),
        (4u64, 2u64, 2u64, 3u64),
        (4u64, 3u64, 1u64, 2u64),
        (u64::MAX, 0u64, 1u64, 0u64),
        (8u64, 7u64, 2u64, 7u64),
    ];
    for (logical_length, window_start, window_count, index) in cases {
        let first = InMemoryRegion::new(7, 1, logical_length, window_start, window_count);
        let second = InMemoryRegion::new(7, 1, logical_length, window_start, window_count);
        assert_eq!(
            first.as_ref().map(|region| region.descriptor()),
            second.as_ref().map(|region| region.descriptor())
        );
        match (first, second) {
            (Ok(mut first), Ok(mut second)) => {
                let first_view = first.borrow_abs(7, first.descriptor().generation);
                let second_view = second.borrow_abs(7, second.descriptor().generation);
                assert_eq!(first_view.is_ok(), second_view.is_ok());
                if let (Ok(first_view), Ok(second_view)) = (first_view, second_view) {
                    let mut first_value = [0u8; 1];
                    let mut second_value = [0u8; 1];
                    assert_eq!(
                        first_view.read(index, &mut first_value),
                        second_view.read(index, &mut second_value)
                    );
                    assert_eq!(first_value, second_value);
                }
            }
            (Err(first), Err(second)) => assert_eq!(first, second),
            _ => panic!("identical boundary inputs produced different constructor classes"),
        }
    }
}
